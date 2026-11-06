/** Poses from the server, drawn a little in the past between the two around that moment. */

export type Sample = { t: number; x: number; z: number; h: number; s: number };

export class Track {
  samples: Sample[] = [];

  push(t: number, x: number, z: number, h: number, s: number) {
    this.samples.push({ t, x, z, h, s });
    if (this.samples.length > 10) this.samples.shift();
  }

  /** The pose at `at`, run on a little past the last sample by its speed. */
  at(at: number): Sample | null {
    const s = this.samples;
    if (!s.length) return null;
    let a = s[0];
    let b = s[s.length - 1];
    for (let i = 0; i + 1 < s.length; i++) {
      if (s[i].t <= at && s[i + 1].t >= at) {
        a = s[i];
        b = s[i + 1];
        break;
      }
    }
    if (at >= b.t) {
      const ahead = Math.min(0.35, at - b.t);
      return { t: at, x: b.x + Math.sin(b.h) * b.s * ahead, z: b.z - Math.cos(b.h) * b.s * ahead, h: b.h, s: b.s };
    }
    const k = b.t > a.t ? Math.max(0, Math.min(1, (at - a.t) / (b.t - a.t))) : 1;
    const dh = Math.atan2(Math.sin(b.h - a.h), Math.cos(b.h - a.h));
    return { t: at, x: a.x + (b.x - a.x) * k, z: a.z + (b.z - a.z) * k, h: a.h + dh * k, s: a.s + (b.s - a.s) * k };
  }
}
