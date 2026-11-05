import { HIT, MISSED, type Engine } from './engine';
import { OPEN } from './types';

export const LANE_COLORS = ['#22c55e', '#ef4444', '#eab308', '#3b82f6', '#f97316'];
const DRUM_COLORS = ['#f97316', '#ef4444', '#eab308', '#3b82f6', '#22c55e'];
const OPEN_COLOR = '#a855f7';
const STAR_COLOR = '#67e8f9';
const DEPTH = 3;
const FLASH_MS = 140;
const PRESS_MS = 130;
const BURST_MS = 260;
const MISS_MS = 360;
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

/** How far through a fade that started at `at` is, from 1 down to 0. */
function fade(time: number, at: number, ms: number) {
  const left = 1 - (time - at) / ms;
  return left > 0 && left <= 1 ? left : 0;
}

function burst(c: CanvasRenderingContext2D, x: number, y: number, size: number, color: string, left: number) {
  const grown = 1 - left;
  c.save();
  c.globalAlpha = left;
  c.strokeStyle = color;
  c.lineWidth = 3 + 3 * left;
  c.beginPath();
  c.ellipse(x, y, size * (0.4 + grown * 0.5), size * (0.18 + grown * 0.24), 0, 0, Math.PI * 2);
  c.stroke();
  c.strokeStyle = '#fff';
  c.lineWidth = 2;
  for (let i = 0; i < 8; i++) {
    const angle = (i / 8) * Math.PI * 2 + Math.PI / 8;
    const dx = Math.cos(angle);
    const dy = Math.sin(angle) * 0.55;
    const from = size * (0.25 + grown * 0.35);
    const to = size * (0.45 + grown * 0.6);
    c.beginPath();
    c.moveTo(x + dx * from, y + dy * from);
    c.lineTo(x + dx * to, y + dy * to);
    c.stroke();
  }
  c.globalAlpha = left * 0.8;
  c.fillStyle = '#fff';
  c.beginPath();
  c.ellipse(x, y - size * 0.2 * grown, size * 0.3 * left, size * 0.42 * left, 0, 0, Math.PI * 2);
  c.fill();
  c.restore();
}

function cross(c: CanvasRenderingContext2D, x: number, y: number, size: number, left: number) {
  c.save();
  c.strokeStyle = `rgba(${MISS_COLOR}, ${left})`;
  c.lineWidth = 5;
  c.lineCap = 'round';
  const r = size * 0.22;
  c.beginPath();
  c.moveTo(x - r, y - r);
  c.lineTo(x + r, y + r);
  c.moveTo(x + r, y - r);
  c.lineTo(x - r, y + r);
  c.stroke();
  c.restore();
}

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
    const held = engine.isSustaining(i);
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
  if (drums) {
    const kickY = g.strike + laneWidth * 0.22;
    const pressed = fade(time, engine.presses[0], PRESS_MS);
    const hit = fade(time, engine.flashes[0], FLASH_MS);
    if (engine.autoKick) {
      c.save();
      c.setLineDash([10, 8]);
      c.lineWidth = 4;
      c.strokeStyle = hit > 0 ? `rgba(255,200,140,${0.5 + hit * 0.5})` : 'rgba(249,115,22,0.45)';
      c.beginPath();
      c.moveTo(edge(-1, 0), kickY + 2);
      c.lineTo(edge(1, 0), kickY + 2);
      c.stroke();
      c.restore();
      c.font = '600 11px system-ui, sans-serif';
      c.textAlign = 'left';
      c.textBaseline = 'top';
      c.fillStyle = 'rgba(253,186,116,0.9)';
      c.fillText('AUTO KICK', edge(-1, 0), kickY + 10);
    } else {
      const lit = Math.max(pressed * 0.7, hit);
      c.fillStyle = lit > 0 ? `rgba(255,200,140,${0.4 + lit * 0.6})` : 'rgba(249,115,22,0.35)';
      c.fillRect(edge(-1, 0), kickY, g.width, 4 + 6 * lit);
    }
    const kickBurst = fade(time, engine.flashes[0], BURST_MS);
    if (kickBurst > 0) {
      c.fillStyle = `rgba(255,200,140,${kickBurst * 0.35})`;
      c.fillRect(edge(-1, 0), g.strike - laneWidth * 0.5 * kickBurst, g.width, laneWidth * 0.7 * kickBurst + 4);
    }
    const kickMiss = fade(time, engine.missFlashes[0], MISS_MS);
    if (kickMiss > 0) {
      c.fillStyle = `rgba(${MISS_COLOR}, ${kickMiss * 0.8})`;
      c.fillRect(edge(-1, 0), kickY, g.width, 5);
      cross(c, g.cx, kickY + 2, laneWidth, kickMiss);
    }
  }
  for (let lane = 0; lane < g.lanes; lane++) {
    const realLane = drums ? lane + 1 : lane;
    const x = laneX(g, lane, 0);
    const held = !drums && engine.held & (1 << realLane);
    const pressed = fade(time, engine.presses[realLane], PRESS_MS);
    c.beginPath();
    c.ellipse(x, g.strike, laneWidth * 0.36, laneWidth * 0.16, 0, 0, Math.PI * 2);
    c.lineWidth = 4 + 3 * pressed;
    c.strokeStyle = colors[realLane];
    c.stroke();
    c.fillStyle = 'rgba(0,0,0,0.6)';
    c.fill();
    if (held || pressed > 0) {
      c.globalAlpha = held ? 1 : 0.35 + 0.65 * pressed;
      c.fillStyle = colors[realLane];
      c.fill();
      c.globalAlpha = 1;
    }
    const missed = fade(time, engine.missFlashes[realLane], MISS_MS);
    if (missed > 0) {
      c.beginPath();
      c.ellipse(x, g.strike, laneWidth * 0.46, laneWidth * 0.22, 0, 0, Math.PI * 2);
      c.lineWidth = 6;
      c.strokeStyle = `rgba(${MISS_COLOR}, ${missed})`;
      c.stroke();
      cross(c, x, g.strike, laneWidth, missed);
    }
    const hit = fade(time, engine.flashes[realLane], BURST_MS);
    if (hit > 0) burst(c, x, g.strike, laneWidth, colors[realLane], hit);
  }
  const overstrum = fade(time, engine.overstrumAt, MISS_MS);
  if (overstrum > 0) {
    const glow = c.createLinearGradient(0, g.strike - laneWidth * 0.6, 0, g.strike + laneWidth * 0.3);
    glow.addColorStop(0, `rgba(${MISS_COLOR}, 0)`);
    glow.addColorStop(1, `rgba(${MISS_COLOR}, ${0.45 * overstrum})`);
    c.fillStyle = glow;
    c.fillRect(edge(-1, 0), g.strike - laneWidth * 0.6, g.width, laneWidth * 0.9);
    const frets = [0, 1, 2, 3, 4].filter((l) => engine.held & (1 << l));
    if (frets.length) for (const lane of frets) cross(c, laneX(g, lane, 0), g.strike, laneWidth, overstrum);
    else cross(c, g.cx, g.strike, laneWidth, overstrum);
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
    c.globalAlpha = missed ? 0.35 : 1;
    if (note.lanes & OPEN || (drums && note.lanes & 1)) {
      const color = missed ? '#555' : star ? STAR_COLOR : drums ? DRUM_COLORS[0] : OPEN_COLOR;
      const half = g.width * s * 0.48;
      if (engine.isAutoKick(i)) {
        c.strokeStyle = color;
        c.lineWidth = 2;
        c.setLineDash([8 * s, 6 * s]);
        c.strokeRect(g.cx - half, py - 3 * s, half * 2, 6 * s);
        c.setLineDash([]);
      } else {
        c.fillStyle = color;
        c.fillRect(g.cx - half, py - 5 * s, half * 2, 10 * s);
      }
      if (!drums || note.lanes === 1) {
        c.globalAlpha = 1;
        continue;
      }
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
    c.globalAlpha = 1;
  }
}
