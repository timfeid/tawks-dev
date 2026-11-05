import type { PlotChunk, Shape, SimDelta, SimFrame } from './contract';

/** Every sample a plot holds, oldest first, as the relay sends them: `[t, v1, v2...]`. */
export type PlotLine = { epoch: number; total: number; points: number[][] };

const MAX_POINTS = 600;

/**
 * Rebuilds frames from the relay's deltas and keeps each plot's samples. A
 * delta whose base this does not hold returns false, and the caller asks the
 * relay for a full frame.
 */
export class SimScene {
  frame: SimFrame | null = null;
  plots = new Map<string, PlotLine>();

  push(delta: SimDelta): boolean {
    let shapes: Shape[];
    if (delta.base === null) shapes = [];
    else if (this.frame && this.frame.n === delta.base) shapes = this.frame.shapes.slice();
    else return false;
    if (delta.del.length) {
      const gone = new Set(delta.del);
      shapes = shapes.filter((s) => !gone.has(s.id));
    }
    if (delta.set.length) {
      const at = new Map(shapes.map((s, i) => [s.id, i]));
      for (const s of delta.set) {
        const i = at.get(s.id);
        if (i === undefined) {
          at.set(s.id, shapes.length);
          shapes.push(s);
        } else shapes[i] = s;
      }
    }
    if (delta.base === null) this.plots.clear();
    for (const chunk of delta.plots) this.absorb(chunk);
    this.frame = {
      n: delta.n,
      t: delta.t,
      playing: delta.playing,
      speed: delta.speed,
      params: delta.params,
      shapes,
      readouts: delta.readouts,
      plots: delta.plots,
      notice: delta.notice ?? null,
    };
    return true;
  }

  absorb(chunk: PlotChunk) {
    let line = this.plots.get(chunk.id);
    if (!line || line.epoch !== chunk.epoch) {
      line = { epoch: chunk.epoch, total: 0, points: [] };
      this.plots.set(chunk.id, line);
    }
    const first = chunk.total - chunk.points.length;
    chunk.points.forEach((point, i) => {
      if (first + i >= line.total) line.points.push(point);
    });
    line.total = Math.max(line.total, chunk.total);
    if (line.points.length > MAX_POINTS) line.points.splice(0, line.points.length - MAX_POINTS);
  }

  reset() {
    this.frame = null;
    this.plots.clear();
  }
}

const lerp = (a: number, b: number, k: number) => a + (b - a) * k;
const lerpOpt = (a: number | null | undefined, b: number | null | undefined, k: number) =>
  a === null || a === undefined || b === null || b === undefined ? b : lerp(a, b, k);

/** A shape partway from `a` to `b`, for smooth motion between the 20 frames a second. */
export function blendShape(a: Shape | undefined, b: Shape, k: number): Shape {
  if (!a || a.kind !== b.kind || k >= 1) return b;
  const out: Shape = { ...b, x: lerp(a.x, b.x, k), y: lerp(a.y, b.y, k) };
  out.x2 = lerpOpt(a.x2, b.x2, k);
  out.y2 = lerpOpt(a.y2, b.y2, k);
  out.r = lerpOpt(a.r, b.r, k);
  out.w = lerpOpt(a.w, b.w, k);
  out.h = lerpOpt(a.h, b.h, k);
  if (a.angle !== undefined && a.angle !== null && b.angle !== undefined && b.angle !== null) {
    let d = b.angle - a.angle;
    while (d > Math.PI) d -= 2 * Math.PI;
    while (d < -Math.PI) d += 2 * Math.PI;
    out.angle = a.angle + d * k;
  }
  if (a.points && b.points && a.points.length === b.points.length) {
    out.points = b.points.map((p, i) => [lerp(a.points![i][0], p[0], k), lerp(a.points![i][1], p[1], k)] as [number, number]);
  }
  return out;
}
