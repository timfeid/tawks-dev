import { HIT, MISSED, type Engine } from './engine';
import { OPEN } from './types';

export const LANE_COLORS = ['#22c55e', '#ef4444', '#eab308', '#3b82f6', '#f97316'];
const DRUM_COLORS = ['#f97316', '#ef4444', '#eab308', '#3b82f6', '#22c55e'];
const OPEN_COLOR = '#a855f7';
const STAR_COLOR = '#67e8f9';
const DEPTH = 3;
const FLASH_MS = 140;
const MISS_MS = 280;
const MISS_COLOR = '239, 68, 68';

type Geometry = {
  cx: number;
  strike: number;
  horizon: number;
  width: number;
  lanes: number;
};

const far = 1 / (1 + DEPTH);
const scaleAt = (z: number) => 1 / (1 + DEPTH * z);

function y(g: Geometry, z: number) {
  return g.strike - ((g.strike - g.horizon) * (1 - scaleAt(z))) / (1 - far);
}

/** Lane centre at depth `z`; on drums, lane 1 is the first pad because the kick spans the highway. */
function laneX(g: Geometry, lane: number, z: number) {
  return g.cx + ((lane + 0.5) / g.lanes - 0.5) * g.width * scaleAt(z);
}

export type Frame = {
  engine: Engine;
  beats: { t: number; bar: boolean }[];
  time: number;
  lookahead: number;
};

function firstFrom<T extends { t: number }>(list: T[], time: number) {
  let lo = 0;
  let hi = list.length;
  while (lo < hi) {
    const mid = (lo + hi) >> 1;
    if (list[mid].t < time) lo = mid + 1;
    else hi = mid;
  }
  return lo;
}

/** Draws the note highway for one player. */
export function drawHighway(c: CanvasRenderingContext2D, w: number, h: number, frame: Frame) {
  const { engine, time, lookahead } = frame;
  const drums = engine.drums;
  const g: Geometry = {
    cx: w / 2,
    strike: h * 0.86,
    horizon: h * 0.06,
    width: Math.min(w * 0.7, h * 0.95),
    lanes: drums ? 4 : 5,
  };
  const z = (t: number) => (t - time) / lookahead;
  const colors = drums ? DRUM_COLORS : LANE_COLORS;
  const padLane = (lane: number) => (drums ? lane - 1 : lane);

  c.clearRect(0, 0, w, h);

  const edge = (side: -1 | 1, depth: number) => g.cx + side * 0.5 * g.width * scaleAt(depth);
  const bottom = -0.1;
  c.beginPath();
  c.moveTo(edge(-1, bottom), y(g, bottom));
  c.lineTo(edge(-1, 1), y(g, 1));
  c.lineTo(edge(1, 1), y(g, 1));
  c.lineTo(edge(1, bottom), y(g, bottom));
  c.closePath();
  const surface = c.createLinearGradient(0, y(g, 1), 0, g.strike);
  surface.addColorStop(0, 'rgba(13,13,13,0)');
  surface.addColorStop(0.25, engine.starActive ? 'rgba(14,60,90,0.9)' : 'rgba(20,20,20,0.92)');
  surface.addColorStop(1, engine.starActive ? 'rgba(20,90,130,0.95)' : 'rgba(26,26,26,0.96)');
  c.fillStyle = surface;
  c.fill();

  c.lineWidth = 1;
  c.strokeStyle = 'rgba(255,255,255,0.08)';
  for (let i = 0; i <= g.lanes; i++) {
    const at = (depth: number) => g.cx + (i / g.lanes - 0.5) * g.width * scaleAt(depth);
    c.beginPath();
    c.moveTo(at(bottom), y(g, bottom));
    c.lineTo(at(1), y(g, 1));
    c.stroke();
  }

  for (let i = firstFrom(frame.beats, time - lookahead * 0.1); i < frame.beats.length; i++) {
    const beat = frame.beats[i];
    const depth = z(beat.t);
    if (depth > 1) break;
    c.strokeStyle = beat.bar ? 'rgba(255,255,255,0.28)' : 'rgba(255,255,255,0.1)';
    c.lineWidth = beat.bar ? 2 : 1;
    c.beginPath();
    c.moveTo(edge(-1, depth), y(g, depth));
    c.lineTo(edge(1, depth), y(g, depth));
    c.stroke();
  }

  const notes = engine.notes;
  const start = firstFrom(notes, time - lookahead * 0.6 - 4000);
  let end = start;
  while (end < notes.length && notes[end].t <= time + lookahead) end++;

  for (let i = start; i < end; i++) {
    const note = notes[i];
    if (!note.len) continue;
    const held = engine.sustain?.index === i;
    if (engine.judged[i] === HIT && !held) continue;
    const from = Math.max(0, z(note.t));
    const to = Math.min(1, z(note.t + note.len));
    if (to <= 0) continue;
    const laneList = note.lanes & OPEN ? [-1] : [0, 1, 2, 3, 4].filter((l) => note.lanes & (1 << l));
    for (const lane of laneList) {
      const color = lane < 0 ? OPEN_COLOR : colors[lane];
      const x0 = lane < 0 ? g.cx : laneX(g, padLane(lane), from);
      const x1 = lane < 0 ? g.cx : laneX(g, padLane(lane), to);
      c.strokeStyle = engine.judged[i] === MISSED ? 'rgba(120,120,120,0.5)' : color;
      c.globalAlpha = held ? 1 : 0.75;
      c.lineWidth = (g.width / g.lanes) * 0.12 * scaleAt(from);
      c.lineCap = 'round';
      c.beginPath();
      c.moveTo(x0, y(g, from));
      c.lineTo(x1, y(g, to));
      c.stroke();
      c.globalAlpha = 1;
    }
  }

  const laneWidth = g.width / g.lanes;
  for (let lane = 0; lane < g.lanes; lane++) {
    const realLane = drums ? lane + 1 : lane;
    const x = laneX(g, lane, 0);
    const held = !drums && engine.held & (1 << realLane);
    const flash = Math.max(0, 1 - (time - engine.flashes[realLane]) / FLASH_MS);
    c.beginPath();
    c.ellipse(x, g.strike, laneWidth * 0.36, laneWidth * 0.16, 0, 0, Math.PI * 2);
    c.lineWidth = 4;
    c.strokeStyle = colors[realLane];
    c.stroke();
    c.fillStyle = held ? colors[realLane] : 'rgba(0,0,0,0.6)';
    c.fill();
    const missed = Math.max(0, 1 - (time - engine.missFlashes[realLane]) / MISS_MS);
    if (missed > 0) {
      c.beginPath();
      c.ellipse(x, g.strike, laneWidth * 0.46, laneWidth * 0.22, 0, 0, Math.PI * 2);
      c.lineWidth = 6;
      c.strokeStyle = `rgba(${MISS_COLOR}, ${missed})`;
      c.stroke();
    }
    if (flash > 0) {
      c.globalAlpha = flash;
      c.fillStyle = '#fff';
      c.beginPath();
      c.ellipse(x, g.strike - laneWidth * 0.25 * (1 - flash), laneWidth * 0.3, laneWidth * 0.5 * flash, 0, 0, Math.PI * 2);
      c.fill();
      c.globalAlpha = 1;
    }
  }
  const overstrum = Math.max(0, 1 - (time - engine.overstrumAt) / MISS_MS);
  if (overstrum > 0) {
    const glow = c.createLinearGradient(0, g.strike - laneWidth * 0.6, 0, g.strike + laneWidth * 0.3);
    glow.addColorStop(0, `rgba(${MISS_COLOR}, 0)`);
    glow.addColorStop(1, `rgba(${MISS_COLOR}, ${0.45 * overstrum})`);
    c.fillStyle = glow;
    c.fillRect(edge(-1, 0), g.strike - laneWidth * 0.6, g.width, laneWidth * 0.9);
  }
  if (drums) {
    const flash = Math.max(0, 1 - (time - engine.flashes[0]) / FLASH_MS);
    c.fillStyle = flash > 0 ? `rgba(255,200,140,${0.4 + flash * 0.6})` : 'rgba(249,115,22,0.35)';
    c.fillRect(edge(-1, 0), g.strike + laneWidth * 0.2, g.width, 4);
  }

  for (let i = end - 1; i >= start; i--) {
    const note = notes[i];
    const state = engine.judged[i];
    if (state === HIT) continue;
    const depth = z(note.t);
    if (depth < -0.04 || depth > 1) continue;
    const s = scaleAt(depth);
    const py = y(g, depth);
    const star = engine.inStarPhrase(i);
    const missed = state === MISSED;
    if (note.lanes & OPEN || (drums && note.lanes & 1)) {
      const color = missed ? '#555' : star ? STAR_COLOR : drums ? DRUM_COLORS[0] : OPEN_COLOR;
      c.fillStyle = color;
      const half = g.width * s * 0.48;
      c.fillRect(g.cx - half, py - 5 * s, half * 2, 10 * s);
      if (drums && note.lanes === 1) continue;
      if (!drums) continue;
    }
    for (let lane = drums ? 1 : 0; lane < 5; lane++) {
      if (!(note.lanes & (1 << lane))) continue;
      const x = laneX(g, padLane(lane), depth);
      const rx = laneWidth * 0.36 * s;
      const ry = laneWidth * 0.17 * s;
      const color = missed ? '#555' : star ? STAR_COLOR : colors[lane];
      c.beginPath();
      c.ellipse(x, py, rx, ry, 0, 0, Math.PI * 2);
      if (note.kind === 'tap') {
        c.fillStyle = '#111';
        c.fill();
        c.lineWidth = 4 * s;
        c.strokeStyle = color;
        c.stroke();
        continue;
      }
      c.fillStyle = color;
      c.fill();
      c.beginPath();
      c.ellipse(x, py - ry * 0.15, rx * (note.kind === 'hopo' ? 0.42 : 0.6), ry * 0.45, 0, 0, Math.PI * 2);
      c.fillStyle = note.kind === 'hopo' ? '#fff' : 'rgba(0,0,0,0.35)';
      c.fill();
    }
  }
}
