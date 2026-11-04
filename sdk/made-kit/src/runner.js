// Runs one round of a game: setup, a fixed 60 Hz tick, bots for bot seats and
// the view. made-simd drives it inside its QuickJS sandbox, and node tests
// drive it directly. It is kit code, not game code.

import { createView, seeded, BUTTONS } from './core.js';

export const TICK_HZ = 60;
export const FRAME_HZ = 20;
export const DT = 1 / TICK_HZ;
const MAX_EVENTS_PER_FRAME = 64;

const IDLE = Object.freeze({ move: { x: 0, z: 0 }, aim: null, held: {}, presses: {} });

function cleanStick(m) {
  if (!m || typeof m !== 'object') return { x: 0, z: 0 };
  let x = Number.isFinite(m.x) ? m.x : 0;
  let z = Number.isFinite(m.z) ? m.z : 0;
  const l = Math.hypot(x, z);
  if (l > 1) { x /= l; z /= l; }
  return { x, z };
}

function cleanAim(a) {
  if (!a || typeof a !== 'object' || !Number.isFinite(a.x) || !Number.isFinite(a.z)) return null;
  return { x: a.x, z: a.z };
}

/**
 * `seats` is [{seat, name, bot, skill, body: {radius, mass, speed}}], skill
 * from 0 to 1. Human input arrives through `input(seat, raw)` as the
 * protocol's Input: {move: {x, z}, aim: {x, z} | null, held: {action, alt},
 * presses: {action, alt}} where presses count every press so far, so a tap
 * shorter than a tick still registers.
 */
export function createRunner(game, { seats, seed = 1 }) {
  const rand = seeded(seed);
  const seatList = seats.map((s) => ({
    seat: s.seat, name: s.name, bot: !!s.bot, skill: s.skill === undefined ? 0.4 : s.skill,
    body: { radius: 1, mass: 1, speed: 1, ...(s.body || {}) },
  }));
  const bySeat = new Map(seatList.map((s) => [s.seat, s]));
  const raw = new Map();
  const last = new Map();
  const botOut = new Map();
  const botActed = {};
  let events = [];
  let time = 0;
  let tick = 0;
  let frameN = 0;
  let placements = null;

  const skill = (seat) => (bySeat.get(seat) ? bySeat.get(seat).skill : 0.4);
  const rosterView = seatList.map((s) => ({ seat: s.seat, name: s.name, bot: s.bot }));

  const inputFor = (seat) => {
    const s = bySeat.get(seat);
    if (!s) return frameInput(IDLE, IDLE, false);
    if (s.bot) {
      const out = botOut.get(seat) || {};
      return {
        move: cleanStick(out.move),
        aim: cleanAim(out.aim),
        held: (a) => !!out[a],
        pressed: (a) => !!out[a],
      };
    }
    return frameInput(raw.get(seat) || IDLE, last.get(seat) || IDLE, true);
  };

  function frameInput(now, before, human) {
    return {
      move: cleanStick(now.move),
      aim: cleanAim(now.aim),
      held: (a) => !!(now.held && now.held[a]),
      pressed: (a) =>
        human &&
        (((now.presses && now.presses[a]) || 0) > ((before.presses && before.presses[a]) || 0) ||
          (!!(now.held && now.held[a]) && !(before.held && before.held[a]))),
    };
  }

  const setupCtx = { seats: rosterView.map((r) => ({ ...r, skill: skill(r.seat), body: { ...bySeat.get(r.seat).body } })), rand };
  let state = game.setup(setupCtx);
  if (state === undefined || state === null || typeof state !== 'object') throw new Error('setup: return the state object');

  const tickCtx = {
    dt: DT,
    get time() { return time; },
    get tick() { return tick; },
    seats: setupCtx.seats,
    rand,
    skill,
    input: inputFor,
    emit(event) {
      if (events.length < MAX_EVENTS_PER_FRAME * 4) events.push(event);
    },
    end(p) {
      if (placements === null) placements = p;
    },
  };

  const botCtx = {
    dt: DT,
    get time() { return time; },
    seats: setupCtx.seats,
    rand,
    skill,
  };

  return {
    meta: game.meta,

    input(seat, value) {
      const s = bySeat.get(seat);
      if (!s || s.bot) return;
      raw.set(seat, value && typeof value === 'object' ? value : IDLE);
    },

    /** Advances one tick. Returns the placements once the game has ended, else null. */
    step() {
      if (placements !== null) return placements;
      for (const s of seatList) {
        if (!s.bot) continue;
        const out = game.bot(state, s.seat, botCtx) || {};
        botOut.set(s.seat, out);
        if ((out.action || out.alt) && botActed[s.seat] === undefined) botActed[s.seat] = time;
      }
      game.tick(state, tickCtx);
      for (const s of seatList) if (!s.bot) last.set(s.seat, raw.get(s.seat) || IDLE);
      tick += 1;
      time = tick * DT;
      return placements;
    },

    /** The scene now, with the one-off effects of every event since the last frame. */
    frame() {
      const v = createView({ n: frameN++, t: time, seats: rosterView });
      game.view(state, v);
      if (game.effect) for (const e of events.slice(0, MAX_EVENTS_PER_FRAME)) game.effect(e, v);
      events = [];
      return v.build();
    },

    get time() { return time; },
    /** When each bot seat first pressed action or alt, in seconds, for the gates. */
    stats: () => ({ acted: botActed }),
    get placements() { return placements; },
    state: () => state,
    restore(json) {
      state = JSON.parse(json);
    },
    buttons: BUTTONS,
  };
}
