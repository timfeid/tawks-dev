import { HIT, MISSED } from './engine';
import type { VocalEngine } from './vocals';

const PAST_MS = 1_200;
const AHEAD_MS = 3_600;
const NOW_X = 0.24;
const VOICE_COLOR = '#67e8f9';
const MISS_COLOR = 'rgba(239, 68, 68, 0.55)';

export type VocalLane = { top: number; height: number };

/** The pitch range to draw, from the song's sung notes with a little room around them. */
export function pitchRange(engine: VocalEngine) {
  const pitched = engine.notes.filter((n) => !n.talky).map((n) => n.pitch);
  if (!pitched.length) return { low: 55, high: 72 };
  return { low: Math.min(...pitched) - 2, high: Math.max(...pitched) + 2 };
}

/** Draws the scrolling pitch lane, Rock Band style, and the singer's voice against it. */
export function drawVocals(
  c: CanvasRenderingContext2D,
  w: number,
  h: number,
  engine: VocalEngine,
  time: number,
  range: { low: number; high: number },
  primary: string,
): VocalLane {
  const top = h * 0.16;
  const height = h * 0.34;
  const nowX = w * NOW_X;
  const x = (t: number) => nowX + ((t - time) / AHEAD_MS) * (w - nowX);
  const span = Math.max(6, range.high - range.low);
  const y = (midi: number) => top + height - ((midi - range.low) / span) * height;
  const row = height / span;

  c.clearRect(0, 0, w, h);
  c.fillStyle = 'rgba(20, 20, 20, 0.92)';
  c.fillRect(0, top - row, w, height + row * 2);
  c.strokeStyle = 'rgba(255, 255, 255, 0.05)';
  c.lineWidth = 1;
  for (let m = Math.ceil(range.low); m <= range.high; m++) {
    if (m % 12 !== 0 && m % 12 !== 7) continue;
    c.beginPath();
    c.moveTo(0, y(m));
    c.lineTo(w, y(m));
    c.stroke();
  }

  const thickness = Math.max(6, Math.min(16, row * 0.8));
  engine.notes.forEach((note, i) => {
    const x0 = x(note.t);
    const x1 = x(note.t + note.len);
    if (x1 < 0 || x0 > w) return;
    const ny = note.talky ? top + height / 2 : y(note.pitch);
    const state = engine.judged[i];
    const share = Math.min(1, engine.credit[i] / Math.max(note.len, 1));
    c.lineCap = 'round';
    c.lineWidth = thickness;
    if (note.talky) c.setLineDash([thickness * 0.6, thickness * 0.6]);
    c.strokeStyle = state === MISSED ? MISS_COLOR : 'rgba(255, 255, 255, 0.28)';
    c.beginPath();
    c.moveTo(x0, ny);
    c.lineTo(x1, ny);
    c.stroke();
    c.setLineDash([]);
    if (share > 0 || state === HIT) {
      c.strokeStyle = primary;
      c.beginPath();
      c.moveTo(x0, ny);
      c.lineTo(x0 + (x1 - x0) * (state === HIT ? Math.max(share, 0.5) : share), ny);
      c.stroke();
    }
  });

  c.strokeStyle = VOICE_COLOR;
  c.lineWidth = 3;
  c.lineCap = 'round';
  c.beginPath();
  let drawing = false;
  for (const sample of engine.trail) {
    if (sample.time < time - PAST_MS || sample.shown === null) {
      drawing = false;
      continue;
    }
    const px = x(sample.time);
    const py = y(sample.shown);
    if (drawing) c.lineTo(px, py);
    else c.moveTo(px, py);
    drawing = true;
  }
  c.stroke();

  c.strokeStyle = 'rgba(255, 255, 255, 0.5)';
  c.lineWidth = 2;
  c.beginPath();
  c.moveTo(nowX, top - row);
  c.lineTo(nowX, top + height + row);
  c.stroke();

  const latest = engine.trail[engine.trail.length - 1];
  if (latest && latest.shown !== null && latest.time > time - 250) {
    const py = y(latest.shown);
    c.fillStyle = VOICE_COLOR;
    c.beginPath();
    c.moveTo(nowX + 10, py);
    c.lineTo(nowX - 6, py - 8);
    c.lineTo(nowX - 6, py + 8);
    c.closePath();
    c.fill();
  }
  return { top: top - row, height: height + row * 2 };
}
