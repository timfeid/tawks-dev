/** Writes small vector tiles for tests. */

type Value = string | number | boolean;
export type TestFeature = { type: 1 | 2 | 3; props: Record<string, Value>; parts: number[][]; closed?: boolean };

function varint(out: number[], n: number) {
  n = n >>> 0 === n ? n : n >>> 0;
  while (n > 127) {
    out.push((n & 127) | 128);
    n = Math.floor(n / 128);
  }
  out.push(n);
}

const zig = (n: number) => (n << 1) ^ (n >> 31);

function field(out: number[], num: number, bytes: number[]) {
  varint(out, (num << 3) | 2);
  varint(out, bytes.length);
  out.push(...bytes);
}

function str(s: string) {
  return [...new TextEncoder().encode(s)];
}

function geometry(f: TestFeature): number[] {
  const cmds: number[] = [];
  let x = 0;
  let y = 0;
  for (const part of f.parts) {
    const pts = part.length / 2;
    const closed = f.type === 3;
    const count = closed ? pts - 1 : pts;
    cmds.push((1 << 3) | 1, zig(part[0] - x), zig(part[1] - y));
    x = part[0];
    y = part[1];
    cmds.push(((count - 1) << 3) | 2);
    for (let i = 1; i < count; i++) {
      cmds.push(zig(part[i * 2] - x), zig(part[i * 2 + 1] - y));
      x = part[i * 2];
      y = part[i * 2 + 1];
    }
    if (closed) cmds.push((1 << 3) | 7);
  }
  return cmds;
}

export function encodeTile(layers: Record<string, TestFeature[]>, extent = 4096): Uint8Array {
  const tile: number[] = [];
  for (const [name, features] of Object.entries(layers)) {
    const keys: string[] = [];
    const values: Value[] = [];
    const layer: number[] = [];
    varint(layer, (15 << 3) | 0);
    varint(layer, 2);
    field(layer, 1, str(name));
    for (const f of features) {
      const tags: number[] = [];
      for (const [k, v] of Object.entries(f.props)) {
        let ki = keys.indexOf(k);
        if (ki < 0) ki = keys.push(k) - 1;
        let vi = values.indexOf(v);
        if (vi < 0) vi = values.push(v) - 1;
        tags.push(ki, vi);
      }
      const feat: number[] = [];
      const packedTags: number[] = [];
      tags.forEach((t) => varint(packedTags, t));
      field(feat, 2, packedTags);
      varint(feat, (3 << 3) | 0);
      varint(feat, f.type);
      const packedGeom: number[] = [];
      geometry(f).forEach((c) => varint(packedGeom, c));
      field(feat, 4, packedGeom);
      field(layer, 2, feat);
    }
    for (const k of keys) field(layer, 3, str(k));
    for (const v of values) {
      const val: number[] = [];
      if (typeof v === 'string') field(val, 1, str(v));
      else if (typeof v === 'boolean') {
        varint(val, (7 << 3) | 0);
        varint(val, v ? 1 : 0);
      } else if (Number.isInteger(v) && v >= 0) {
        varint(val, (5 << 3) | 0);
        varint(val, v);
      } else {
        varint(val, (3 << 3) | 1);
        const b = new Uint8Array(8);
        new DataView(b.buffer).setFloat64(0, v, true);
        val.push(...b);
      }
      field(layer, 4, val);
    }
    varint(layer, (5 << 3) | 0);
    varint(layer, extent);
    field(tile, 3, layer);
  }
  return new Uint8Array(tile);
}
