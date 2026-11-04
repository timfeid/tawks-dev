// The made kit core: what a game.js imports. Everything here is pure and runs
// the same in the server sandbox (QuickJS), in node and in the browser. It
// never touches the DOM, the clock or Math.random.
//
// Conventions: the floor is the x/z plane, y is up, distances are metres and
// time is seconds. A yaw of 0 faces +z and grows towards +x, so
// `Math.atan2(dx, dz)` is the yaw that faces along (dx, dz).

export const KIT_VERSION = 1;

export const COLORS = [
  'ice', 'water', 'danger', 'seat', 'gold', 'wood', 'snow', 'stone', 'grass', 'sand', 'lava',
  'light', 'dark', 'red', 'orange', 'yellow', 'green', 'blue', 'purple', 'pink',
];
export const POSES = ['idle', 'ride', 'cheer', 'sad', 'fall', 'sit', 'wave', 'play'];
export const GROUNDS = ['grass', 'stone', 'snow', 'wood', 'sand'];
export const SOUNDS = [
  'bump', 'hit', 'splash', 'crack', 'coin', 'win', 'lose', 'tick', 'bell', 'power', 'thud', 'pop',
  'blocked', 'confirm',
];
export const CONTROLS = ['move', 'move_action', 'move_aim'];
/** 'screen': each key moves that way on screen. 'tank': up is throttle, down reverse, left and right turn the hull. */
export const STEERS = ['screen', 'tank'];
export const BUTTONS = ['action', 'alt'];
export const INPUTS = ['up', 'down', 'left', 'right', 'action'];
export const SKILLS = { relaxed: 0.15, steady: 0.4, sharp: 0.7, ruthless: 1 };
export const MAX_ENTITIES = 400;
export const MAX_SEATS = 8;

const fail = (where, message) => {
  throw new Error(`${where}: ${message}`);
};

const isFiniteNumber = (n) => typeof n === 'number' && Number.isFinite(n);

/** Declares a game. Checks its shape so a broken game fails at load, not mid-round. */
export function defineGame(game) {
  if (!game || typeof game !== 'object') fail('defineGame', 'pass one object: { meta, setup, tick, bot, view, effect }');
  const meta = game.meta;
  if (!meta || typeof meta !== 'object') fail('defineGame', 'meta is missing');
  if (typeof meta.name !== 'string' || !meta.name.trim()) fail('meta.name', 'give the game a name');
  if (typeof meta.rules !== 'string' || !meta.rules.trim()) fail('meta.rules', 'say how to play in a sentence or two');
  const players = meta.players;
  if (!Array.isArray(players) || players.length !== 2 || !players.every(Number.isInteger))
    fail('meta.players', 'use [min, max], e.g. [2, 8]');
  if (players[0] < 1 || players[1] > MAX_SEATS || players[0] > players[1])
    fail('meta.players', `min and max must be between 1 and ${MAX_SEATS}, min first`);
  if (!CONTROLS.includes(meta.controls)) fail('meta.controls', `use one of ${CONTROLS.join(', ')}`);
  if (!isFiniteNumber(meta.seconds) || meta.seconds < 5 || meta.seconds > 600)
    fail('meta.seconds', 'the longest a round may run, between 5 and 600');
  for (const key of ['goal', 'win', 'lose']) {
    const value = meta[key];
    if (value === undefined) continue;
    if (typeof value !== 'string' || !value.trim()) fail(`meta.${key}`, 'one short line of text');
    if (value.length > 90) fail(`meta.${key}`, 'keep it to one short line, at most 90 characters');
  }
  if (meta.steer !== undefined && !STEERS.includes(meta.steer)) fail('meta.steer', `use one of ${STEERS.join(', ')}`);
  if (meta.unused !== undefined) {
    if (!Array.isArray(meta.unused) || !meta.unused.every((u) => INPUTS.includes(u)))
      fail('meta.unused', `a list of the inputs the game ignores, from ${INPUTS.join(', ')}`);
  }
  for (const hook of ['setup', 'tick', 'bot', 'view'])
    if (typeof game[hook] !== 'function') fail('defineGame', `${hook} must be a function`);
  if (game.effect !== undefined && typeof game.effect !== 'function') fail('defineGame', 'effect must be a function when given');
  return game;
}

// ---------------------------------------------------------------- helpers

export const clamp = (n, lo, hi) => Math.min(hi, Math.max(lo, n));
export const lerp = (a, b, t) => a + (b - a) * t;

/** Length of a vector {x, z}. */
export const len = (v) => Math.hypot(v.x, v.z);

/** The vector scaled to length 1, or {0, 0} for a zero vector. */
export function norm(v) {
  const l = Math.hypot(v.x, v.z);
  return l > 1e-9 ? { x: v.x / l, z: v.z / l } : { x: 0, z: 0 };
}

/** `n` spawn points evenly around a circle, the first at -z (the far side). */
export function ring(n, radius, phase = 0) {
  const out = [];
  for (let i = 0; i < n; i++) {
    const a = phase + Math.PI + (i * Math.PI * 2) / Math.max(1, n);
    out.push({ x: Math.sin(a) * radius, z: Math.cos(a) * radius });
  }
  return out;
}

/** Tile centres of a cols × rows grid of `size` tiles, centred on the origin. */
export function grid(cols, rows, size) {
  const out = [];
  for (let r = 0; r < rows; r++)
    for (let c = 0; c < cols; c++)
      out.push({ col: c, row: r, x: (c - (cols - 1) / 2) * size, z: (r - (rows - 1) / 2) * size });
  return out;
}

/** A round physics body. `r` is its radius, `mass` how hard it is to shove. */
export function body(x, z, { r = 0.6, mass = 1, yaw = 0 } = {}) {
  return { x, z, vx: 0, vz: 0, r, mass, yaw };
}

/**
 * Moves a body towards `move` (a stick, length up to 1) at up to `speed`.
 * It speeds up at `accel` m/s², and with no input it slows at
 * `accel * friction`, so a low friction feels like ice.
 */
export function steer(b, move, dt, { speed = 6, accel = 20, friction = 0.5 } = {}) {
  let mx = move ? move.x || 0 : 0;
  let mz = move ? move.z || 0 : 0;
  const m = Math.hypot(mx, mz);
  if (m > 1) { mx /= m; mz /= m; }
  const active = m > 0.05;
  const tx = active ? mx * speed : 0;
  const tz = active ? mz * speed : 0;
  const dvx = tx - b.vx;
  const dvz = tz - b.vz;
  const dv = Math.hypot(dvx, dvz);
  const step = (active ? accel : accel * friction) * dt;
  if (dv <= step) { b.vx = tx; b.vz = tz; } else { b.vx += (dvx / dv) * step; b.vz += (dvz / dv) * step; }
  b.x += b.vx * dt;
  b.z += b.vz * dt;
  if (active) b.yaw = Math.atan2(mx, mz);
  return b;
}

/**
 * Hull steering for boats, karts and anything that slides (pair it with
 * meta.steer = 'tank'). Up on the stick is throttle along the way the body
 * faces, down is reverse or brake, left and right turn the hull (right turns
 * clockwise on screen, so the yaw falls). The heading turns at `turn` rad/s,
 * but the velocity only swings round to it at `grip` per second, so a low
 * grip slides wide on turns. Turning needs some speed, like a real hull.
 * Off the throttle it coasts to a stop in about two seconds; down brakes.
 */
export function drift(b, move, dt, { thrust = 14, reverse = 6, brake = 14, turn = 1.9, grip = 1.4, speed = 9, drag = 0.5, coast = 1.6 } = {}) {
  const throttle = move ? -(move.z || 0) : 0;
  const steerIn = move ? move.x || 0 : 0;
  const sp = Math.hypot(b.vx, b.vz);
  const authority = Math.min(1, 0.35 + sp / 3);
  b.yaw = (b.yaw || 0) - steerIn * turn * authority * dt;
  const hx = Math.sin(b.yaw);
  const hz = Math.cos(b.yaw);
  const along = b.vx * hx + b.vz * hz;
  if (throttle > 0) {
    b.vx += hx * throttle * thrust * dt;
    b.vz += hz * throttle * thrust * dt;
  } else if (throttle < 0 && along > 0.3) {
    // Down while going forward brakes hard before it reverses.
    const cut = Math.min(along, -throttle * brake * dt);
    b.vx -= hx * cut;
    b.vz -= hz * cut;
  } else if (throttle < 0) {
    b.vx += hx * throttle * reverse * dt;
    b.vz += hz * throttle * reverse * dt;
  }
  const along2 = b.vx * hx + b.vz * hz;
  const k = Math.min(1, grip * dt);
  b.vx -= (b.vx - hx * along2) * k;
  b.vz -= (b.vz - hz * along2) * k;
  // Off the throttle the water slows the hull quickly, so it settles within a couple of seconds.
  const d = Math.exp(-(drag + (Math.abs(throttle) < 0.05 ? coast : 0)) * dt);
  b.vx *= d;
  b.vz *= d;
  const s = Math.hypot(b.vx, b.vz);
  if (s > speed) { b.vx *= speed / s; b.vz *= speed / s; }
  b.x += b.vx * dt;
  b.z += b.vz * dt;
  return b;
}

/** A burst of speed along `dir`, or along the way the body faces when `dir` is zero. */
export function dash(b, dir, power) {
  let d = dir ? norm(dir) : { x: 0, z: 0 };
  if (d.x === 0 && d.z === 0) d = { x: Math.sin(b.yaw || 0), z: Math.cos(b.yaw || 0) };
  b.vx = d.x * power;
  b.vz = d.z * power;
  b.yaw = Math.atan2(d.x, d.z);
  return b;
}

/**
 * Pushes overlapping bodies apart and bounces them off each other, the
 * lighter one moving more. `bounce` 0 is a dead stop, 1 a perfect bounce.
 * Returns the pairs that touched this call, as [a, b].
 */
export function collide(bodies, { bounce = 0.8 } = {}) {
  const hits = [];
  for (let i = 0; i < bodies.length; i++) {
    for (let j = i + 1; j < bodies.length; j++) {
      const a = bodies[i];
      const b = bodies[j];
      let dx = b.x - a.x;
      let dz = b.z - a.z;
      let d = Math.hypot(dx, dz);
      const reach = a.r + b.r;
      if (d >= reach) continue;
      if (d < 1e-6) { dx = 1; dz = 0; d = 0; } else { dx /= d; dz /= d; }
      const ma = a.mass || 1;
      const mb = b.mass || 1;
      const overlap = reach - d;
      const pa = mb / (ma + mb);
      const pb = ma / (ma + mb);
      a.x -= dx * overlap * pa; a.z -= dz * overlap * pa;
      b.x += dx * overlap * pb; b.z += dz * overlap * pb;
      const rel = (b.vx - a.vx) * dx + (b.vz - a.vz) * dz;
      if (rel < 0) {
        const impulse = (-(1 + bounce) * rel) / (1 / ma + 1 / mb);
        a.vx -= (impulse / ma) * dx; a.vz -= (impulse / ma) * dz;
        b.vx += (impulse / mb) * dx; b.vz += (impulse / mb) * dz;
      }
      hits.push([a, b]);
    }
  }
  return hits;
}

/** Keeps a body inside a shape, bouncing it off the edge. Returns true when it hit the edge. */
export function keepInside(b, shape, { bounce = 0.5 } = {}) {
  const cx = shape.x || 0;
  const cz = shape.z || 0;
  const r = b.r || 0;
  if (shape.r !== undefined) {
    const dx = b.x - cx;
    const dz = b.z - cz;
    const d = Math.hypot(dx, dz);
    const max = shape.r - r;
    if (d <= max || d < 1e-9) return false;
    const nx = dx / d;
    const nz = dz / d;
    b.x = cx + nx * max;
    b.z = cz + nz * max;
    const out = b.vx * nx + b.vz * nz;
    if (out > 0) { b.vx -= (1 + bounce) * out * nx; b.vz -= (1 + bounce) * out * nz; }
    return true;
  }
  let hit = false;
  const hx = shape.w / 2 - r;
  const hz = shape.d / 2 - r;
  if (b.x < cx - hx) { b.x = cx - hx; if (b.vx < 0) b.vx = -b.vx * bounce; hit = true; }
  if (b.x > cx + hx) { b.x = cx + hx; if (b.vx > 0) b.vx = -b.vx * bounce; hit = true; }
  if (b.z < cz - hz) { b.z = cz - hz; if (b.vz < 0) b.vz = -b.vz * bounce; hit = true; }
  if (b.z > cz + hz) { b.z = cz + hz; if (b.vz > 0) b.vz = -b.vz * bounce; hit = true; }
  return hit;
}

export const dist = (a, b) => Math.hypot(b.x - a.x, b.z - a.z);

/** The yaw from `a` that faces `b`. */
export const angle = (a, b) => Math.atan2(b.x - a.x, b.z - a.z);

/** Is point `p` inside `{x, z, r}` (a circle) or `{x, z, w, d}` (an upright box)? */
export function inside(p, shape) {
  const cx = shape.x || 0;
  const cz = shape.z || 0;
  if (shape.r !== undefined) return Math.hypot(p.x - cx, p.z - cz) <= shape.r;
  return Math.abs(p.x - cx) <= shape.w / 2 && Math.abs(p.z - cz) <= shape.d / 2;
}

/** The item of `list` closest to `from`, or null for an empty list. */
export function nearest(from, list) {
  let best = null;
  let bestD = Infinity;
  for (const item of list) {
    const d = dist(from, item);
    if (d < bestD) { bestD = d; best = item; }
  }
  return best;
}

/**
 * A bot's stick towards `target`. A lower `skill` (0 to 1) wobbles more and
 * moves slower; it eases off when it arrives.
 */
export function seek(b, target, skill = 0.4) {
  const s = clamp(skill, 0, 1);
  const dx = target.x - b.x;
  const dz = target.z - b.z;
  const d = Math.hypot(dx, dz);
  if (d < 0.05) return { x: 0, z: 0 };
  const wobble = (1 - s) * 0.6 * Math.sin(b.x * 1.7 + b.z * 2.3);
  const a = Math.atan2(dx, dz) + wobble;
  const pace = (0.62 + 0.38 * s) * Math.min(1, d / 0.8);
  return { x: Math.sin(a) * pace, z: Math.cos(a) * pace };
}

/** A stick pointing away from `threat`, for running away. */
export function flee(b, threat, skill = 0.4) {
  return seek(b, { x: b.x * 2 - threat.x, z: b.z * 2 - threat.z }, skill);
}

/**
 * Placements from elimination order. Each player has `seat` and `out`: null
 * while still in, else the time they went out. Still-in players share first;
 * later outs place higher; equal outs tie. Returns tied groups of seats,
 * best first, e.g. [[3], [1, 4], [2]].
 */
export function rankOuts(players, key = 'out') {
  return rankBy(players, (p) => (p[key] === null || p[key] === undefined ? Infinity : p[key]));
}

/** Placements by score, highest first (or lowest first with `{ lowWins: true }`). Equal scores tie. */
export function rankBy(players, score, { lowWins = false } = {}) {
  const scored = players.map((p) => ({ seat: p.seat, s: typeof score === 'function' ? score(p) : p[score] }));
  scored.sort((a, b) => (lowWins ? a.s - b.s : b.s - a.s) || a.seat - b.seat);
  const groups = [];
  let last;
  for (const p of scored) {
    if (groups.length && p.s === last) groups[groups.length - 1].push(p.seat);
    else groups.push([p.seat]);
    last = p.s;
  }
  return groups;
}

/** A random item of `list`, using `rand` (ctx.rand). */
export const pick = (rand, list) => list[Math.floor(rand() * list.length)];

/** A shuffled copy of `list`, using `rand` (ctx.rand). */
export function shuffle(rand, list) {
  const out = list.slice();
  for (let i = out.length - 1; i > 0; i--) {
    const j = Math.floor(rand() * (i + 1));
    [out[i], out[j]] = [out[j], out[i]];
  }
  return out;
}

/** A seeded random number generator returning [0, 1). The runner gives games one as ctx.rand. */
export function seeded(seed) {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

// ------------------------------------------------------------ the scene

const round = (n) => Math.round(n * 100) / 100;

function num(where, name, n, fallback) {
  if (n === undefined || n === null) {
    if (fallback === undefined) fail(where, `${name} is missing`);
    return fallback;
  }
  if (!isFiniteNumber(n)) fail(where, `${name} must be a finite number, got ${n}`);
  return round(n);
}

function color(where, c, fallback) {
  if (c === undefined || c === null) return fallback;
  if (!COLORS.includes(c)) fail(where, `unknown color '${c}'; use one of ${COLORS.join(', ')}`);
  return c;
}

function text(where, s, max) {
  if (s === undefined || s === null || s === false) return null;
  if (typeof s !== 'string' && typeof s !== 'number') fail(where, 'must be text');
  const out = String(s);
  return out.length > max ? out.slice(0, max) : out;
}

/**
 * The scene builder a game's `view(state, v)` and `effect(event, v)` draw
 * with. `build()` returns a scene frame (see scene.ts). The runner makes one
 * per frame; games never call this themselves.
 */
export function createView({ n = 0, t = 0, seats = [] } = {}) {
  const entities = [];
  const ids = new Set();
  const effects = [];
  const stage = { water: false, ground: null };
  let camera = null;
  let hud = { objective: null, alert: null, timer: null, scores: [], meter: null };

  const add = (where, e) => {
    if (typeof e.id !== 'string' || !e.id || e.id.length > 48) fail(where, 'the id must be text of 1 to 48 characters');
    if (ids.has(e.id)) fail(where, `the id '${e.id}' is used twice in one frame`);
    if (entities.length >= MAX_ENTITIES) fail(where, `a frame can hold at most ${MAX_ENTITIES} things`);
    ids.add(e.id);
    entities.push(e);
  };
  const flags = (e, opts) => {
    if (opts.hidden) e.hidden = true;
    return e;
  };
  const seatOf = (where, seat) => {
    if (!Number.isInteger(seat) || seat < 1 || seat > MAX_SEATS) fail(where, `seat must be 1 to ${MAX_SEATS}, got ${seat}`);
    return seat;
  };

  const v = {
    time: t,
    frame: n,
    seats,

    player(seat, b, opts = {}) {
      const where = `v.player(${seat})`;
      seatOf(where, seat);
      if (!b) fail(where, 'pass the body, e.g. v.player(p.seat, p)');
      const pose = opts.pose === undefined ? 'idle' : opts.pose;
      if (!POSES.includes(pose)) fail(where, `unknown pose '${pose}'; use one of ${POSES.join(', ')}`);
      add(where, flags({
        id: `p${seat}`, kind: 'player', seat,
        x: num(where, 'x', b.x), y: num(where, 'y', opts.y, 0), z: num(where, 'z', b.z),
        r: num(where, 'r', b.r, 0.6), yaw: num(where, 'yaw', opts.yaw === undefined ? b.yaw : opts.yaw, 0),
        pose,
      }, opts));
    },
    disc(id, x, z, r, opts = {}) {
      const where = `v.disc('${id}')`;
      add(where, flags({
        id: String(id), kind: 'disc',
        x: num(where, 'x', x), y: num(where, 'y', opts.y, 0), z: num(where, 'z', z),
        r: num(where, 'r', r), h: num(where, 'height', opts.height, 0.2),
        color: color(where, opts.color, 'stone'), ...(opts.seat ? { seat: seatOf(where, opts.seat) } : {}),
      }, opts));
    },
    box(id, x, z, w, d, opts = {}) {
      const where = `v.box('${id}')`;
      add(where, flags({
        id: String(id), kind: 'box',
        x: num(where, 'x', x), y: num(where, 'y', opts.y, 0), z: num(where, 'z', z),
        w: num(where, 'w', w), d: num(where, 'd', d), h: num(where, 'h', opts.h, 1),
        yaw: num(where, 'yaw', opts.yaw, 0),
        color: color(where, opts.color, 'wood'), ...(opts.seat ? { seat: seatOf(where, opts.seat) } : {}),
      }, opts));
    },
    ball(id, x, z, r, opts = {}) {
      const where = `v.ball('${id}')`;
      add(where, flags({
        id: String(id), kind: 'ball',
        x: num(where, 'x', x), y: num(where, 'y', opts.y, 0), z: num(where, 'z', z),
        r: num(where, 'r', r), yaw: num(where, 'spin', opts.spin, 0),
        color: color(where, opts.color, 'snow'), ...(opts.seat ? { seat: seatOf(where, opts.seat) } : {}),
      }, opts));
    },
    marker(id, x, z, r, opts = {}) {
      const where = `v.marker('${id}')`;
      add(where, flags({
        id: String(id), kind: 'marker',
        x: num(where, 'x', x), y: num(where, 'y', opts.y, 0), z: num(where, 'z', z),
        r: num(where, 'r', r), color: color(where, opts.color, 'gold'),
        ...(opts.seat ? { seat: seatOf(where, opts.seat) } : {}),
      }, opts));
    },
    water() {
      stage.water = true;
    },
    ground(kind = 'grass') {
      if (!GROUNDS.includes(kind)) fail('v.ground', `unknown ground '${kind}'; use one of ${GROUNDS.join(', ')}`);
      stage.ground = kind;
    },
    camera: {
      frame(halfX, halfZ, pitch = 55, at = {}) {
        camera = {
          half_x: num('v.camera.frame', 'halfX', halfX), half_z: num('v.camera.frame', 'halfZ', halfZ),
          pitch: clamp(num('v.camera.frame', 'pitch', pitch), 20, 90),
          x: num('v.camera.frame', 'x', at.x, 0), z: num('v.camera.frame', 'z', at.z, 0), follow: null,
        };
      },
      follow(seat, { half = 8, pitch = 50 } = {}) {
        camera = {
          half_x: num('v.camera.follow', 'half', half), half_z: num('v.camera.follow', 'half', half),
          pitch: clamp(num('v.camera.follow', 'pitch', pitch), 20, 90), x: 0, z: 0,
          follow: seatOf('v.camera.follow', seat),
        };
      },
    },
    hud(h = {}) {
      const scores = Array.isArray(h.scores) ? h.scores.slice(0, MAX_SEATS).map((s) => {
        const out = { seat: seatOf('v.hud scores', s.seat) };
        if (s.score !== undefined && s.score !== null) out.score = num('v.hud scores', 'score', s.score);
        if (s.up !== undefined && s.up !== null) out.up = !!s.up;
        const label = text('v.hud scores label', s.label, 24);
        if (label !== null) out.label = label;
        return out;
      }) : [];
      let meter = null;
      if (h.meter) {
        meter = {
          label: text('v.hud meter', h.meter.label, 40) || '',
          value: clamp(num('v.hud meter', 'value', h.meter.value), 0, 1),
          color: color('v.hud meter', h.meter.color, 'gold'),
        };
      }
      hud = {
        objective: text('v.hud objective', h.objective, 120),
        alert: text('v.hud alert', h.alert, 80),
        timer: h.timer === undefined || h.timer === null ? null : Math.max(0, num('v.hud timer', 'timer', h.timer)),
        scores,
        meter,
      };
    },
    sound(name) {
      if (!SOUNDS.includes(name)) fail('v.sound', `unknown sound '${name}'; use one of ${SOUNDS.join(', ')}`);
      if (effects.length < 32) effects.push({ kind: 'sound', name });
    },
    burst(x, z, c = 'gold') {
      if (effects.length < 32)
        effects.push({ kind: 'burst', x: num('v.burst', 'x', x), z: num('v.burst', 'z', z), color: color('v.burst', c, 'gold') });
    },

    build() {
      return {
        n,
        t: round(t),
        stage: { ...stage },
        camera: camera || { half_x: 10, half_z: 10, pitch: 55, x: 0, z: 0, follow: null },
        hud,
        entities,
        effects,
      };
    },
  };
  return v;
}

// ------------------------------------------------------------- frames

const sameEntity = (a, b) => {
  for (const k in a) if (a[k] !== b[k]) return false;
  for (const k in b) if (!(k in a)) return false;
  return true;
};

/**
 * The delta that turns frame `prev` into frame `next` (both full scene
 * frames), or a full delta when `prev` is null. See FrameDelta in scene.ts.
 */
export function diffFrame(prev, next) {
  if (!prev) return { n: next.n, t: next.t, base: null, stage: next.stage, camera: next.camera, hud: next.hud, set: next.entities, del: [], effects: next.effects };
  const before = new Map(prev.entities.map((e) => [e.id, e]));
  const set = [];
  for (const e of next.entities) {
    const old = before.get(e.id);
    before.delete(e.id);
    if (!old || !sameEntity(old, e)) set.push(e);
  }
  return { n: next.n, t: next.t, base: prev.n, stage: next.stage, camera: next.camera, hud: next.hud, set, del: [...before.keys()], effects: next.effects };
}

/**
 * Applies a delta to the frame the client holds and returns the new full
 * frame. Returns null when the delta is based on a frame the client does not
 * have; the client then waits for the next full frame.
 */
export function applyFrame(prev, delta) {
  if (delta.base !== null && delta.base !== undefined) {
    if (!prev || prev.n !== delta.base) return null;
  }
  const map = new Map(delta.base === null || delta.base === undefined ? [] : prev.entities.map((e) => [e.id, e]));
  for (const id of delta.del) map.delete(id);
  for (const e of delta.set) map.set(e.id, e);
  return { n: delta.n, t: delta.t, stage: delta.stage, camera: delta.camera, hud: delta.hud, entities: [...map.values()], effects: delta.effects };
}
