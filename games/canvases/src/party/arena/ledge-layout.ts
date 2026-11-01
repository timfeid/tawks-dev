import type { LedgeFooting, LedgeRing } from '../protocol';

export type LedgeSection = { index: number; ring: number; inner: number; outer: number; from: number; to: number };

/** Every section of the ledge, numbered the way the server numbers them. */
export function ledgeSections(rings: LedgeRing[]): LedgeSection[] {
  const sections: LedgeSection[] = [];
  rings.forEach((ring, k) => {
    const span = (Math.PI * 2) / ring.count;
    for (let slot = 0; slot < ring.count; slot++) {
      sections.push({
        index: sections.length,
        ring: k,
        inner: ring.inner,
        outer: ring.outer,
        from: ring.turn + span * slot,
        to: ring.turn + span * (slot + 1),
      });
    }
  });
  return sections;
}

/** The section under a point: -1 on the core, null off the ledge. */
export function ledgeSectionAt(rings: LedgeRing[], core: number, x: number, z: number): number | null {
  const out = Math.hypot(x, z);
  if (out <= core) return -1;
  let first = 0;
  for (const ring of rings) {
    if (out > ring.inner && out <= ring.outer) {
      const tau = Math.PI * 2;
      const around = (((Math.atan2(z, x) - ring.turn) % tau) + tau) % tau;
      return first + Math.min(ring.count - 1, Math.floor(around / (tau / ring.count)));
    }
    first += ring.count;
  }
  return null;
}

/** How far out the ledge still reaches, for framing the shot. */
export function ledgeReach(rings: LedgeRing[], core: number, footing: LedgeFooting[] | undefined) {
  let reach = core;
  let first = 0;
  for (const ring of rings) {
    const left = (footing ?? []).slice(first, first + ring.count);
    if (!footing || left.some((state) => state < 2)) reach = ring.outer;
    first += ring.count;
  }
  return reach;
}

/** The how-to-play line, in the keys or buttons this player actually has. */
export function ledgeHint(move: string[], action: string, pad: string | null) {
  if (pad) return `Left stick to move · ${pad} to shove · let go of the stick to brace`;
  return `${move.join(' ')} to move · ${action} or click to shove · let go of the keys to brace`;
}
