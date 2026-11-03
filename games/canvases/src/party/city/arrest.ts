import type { CityArrest, CityView } from "../protocol";

/** Mirrors city_arrest::figure_eight on the server, in pad units (y up). */
export function figureEight(t: number): [number, number] {
  return [Math.sin(t), Math.sin(2 * t) * 0.6];
}
function tangent(t: number): [number, number] {
  const dx = Math.cos(t),
    dy = 1.2 * Math.cos(2 * t),
    length = Math.hypot(dx, dy) || 1;
  return [dx / length, dy / length];
}
export const LOOP = Math.PI * 2;

export function myArrest(
  city: CityView | null | undefined,
  seat: number | null,
) {
  if (!city || seat === null) return null;
  const arrest = city.arrests.find(
    (a) => a.officer === seat || a.robber === seat,
  );
  if (!arrest) return null;
  return { arrest, officer: arrest.officer === seat };
}

/** Struggle rhythm phase, advanced locally from the last authoritative clock. */
export function strugglePhase(arrest: CityArrest, sinceSnapshot: number) {
  const phase = ((arrest.clock + sinceSnapshot) / arrest.period) % 1;
  return phase < 0 ? phase + 1 : phase;
}
export function inBand(arrest: CityArrest, phase: number) {
  return Math.abs(phase - 0.5) <= arrest.window;
}

const clamp = (v: number) => Math.max(-1.5, Math.min(1.5, v));

/**
 * The officer's cursor on the trace pad. A mouse or touch sets it directly,
 * a controller stick is a position, and keys steer it along the line when
 * they point roughly the same way as the curve.
 */
export class TraceCursor {
  x = 0;
  y = 0;
  private lastPointer = -Infinity;
  reset() {
    this.x = this.y = 0;
    this.lastPointer = -Infinity;
  }
  pointer(x: number, y: number) {
    this.x = clamp(x);
    this.y = clamp(y);
    this.lastPointer = performance.now();
  }
  pointerDelta(dx: number, dy: number) {
    this.pointer(this.x + dx, this.y + dy);
  }
  stick(x: number, y: number) {
    this.x = clamp(x * 1.1);
    this.y = clamp(y * 1.1);
  }
  keys(x: number, y: number, dt: number, guide: number) {
    if (!x && !y) return;
    if (performance.now() - this.lastPointer < 400) return;
    const length = Math.hypot(x, y);
    let dx = x / length,
      dy = y / length;
    const [tx, ty] = tangent(guide);
    const along = dx * tx + dy * ty;
    if (along > 0.5) {
      dx = tx;
      dy = ty;
      // Pull back onto the line so held keys follow the curve, not a chord.
      const [gx, gy] = figureEight(guide);
      this.x += (gx - this.x) * Math.min(1, dt * 6);
      this.y += (gy - this.y) * Math.min(1, dt * 6);
    }
    this.x = clamp(this.x + dx * 2.3 * dt);
    this.y = clamp(this.y + dy * 2.3 * dt);
  }
}
