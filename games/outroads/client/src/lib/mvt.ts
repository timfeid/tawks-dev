/** A Mapbox Vector Tile reader, enough for the OpenMapTiles layers Outroads draws. */

export type Props = Record<string, string | number | boolean>;
export type Ring = number[];

export type Feature = {
  type: 1 | 2 | 3;
  props: Props;
  /** Each part as flat [x, y, x, y, ...] in tile units, y down. */
  parts: Ring[];
};

export type Layer = { name: string; extent: number; features: Feature[] };

class Reader {
  pos = 0;
  constructor(private buf: Uint8Array) {}

  get done() {
    return this.pos >= this.buf.length;
  }

  varint(): number {
    let result = 0;
    let shift = 0;
    let byte: number;
    do {
      byte = this.buf[this.pos++];
      if (shift < 28) result |= (byte & 0x7f) << shift;
      else result += (byte & 0x7f) * 2 ** shift;
      shift += 7;
    } while (byte & 0x80);
    return result;
  }

  bytes(): Uint8Array {
    const len = this.varint();
    const out = this.buf.subarray(this.pos, this.pos + len);
    this.pos += len;
    return out;
  }

  skip(wire: number) {
    if (wire === 0) this.varint();
    else if (wire === 1) this.pos += 8;
    else if (wire === 2) this.pos += this.varint();
    else if (wire === 5) this.pos += 4;
    else throw new Error(`Unknown wire type ${wire}`);
  }

  view() {
    return new DataView(this.buf.buffer, this.buf.byteOffset, this.buf.byteLength);
  }
}

const text = new TextDecoder();
const zigzag = (n: number) => (n >>> 1) ^ -(n & 1);

function packed(bytes: Uint8Array): number[] {
  const r = new Reader(bytes);
  const out: number[] = [];
  while (!r.done) out.push(r.varint());
  return out;
}

function readValue(bytes: Uint8Array): string | number | boolean {
  const r = new Reader(bytes);
  let value: string | number | boolean = '';
  while (!r.done) {
    const tag = r.varint();
    const field = tag >> 3;
    const wire = tag & 7;
    if (field === 1) value = text.decode(r.bytes());
    else if (field === 2) {
      value = r.view().getFloat32(r.pos, true);
      r.pos += 4;
    } else if (field === 3) {
      value = r.view().getFloat64(r.pos, true);
      r.pos += 8;
    } else if (field === 4 || field === 5) value = r.varint();
    else if (field === 6) value = zigzag(r.varint());
    else if (field === 7) value = r.varint() !== 0;
    else r.skip(wire);
  }
  return value;
}

export function decodeGeometry(commands: number[]): Ring[] {
  const parts: Ring[] = [];
  let current: Ring | null = null;
  let x = 0;
  let y = 0;
  let i = 0;
  while (i < commands.length) {
    const word = commands[i++];
    const cmd = word & 7;
    const count = word >> 3;
    if (cmd === 7) {
      if (current && current.length >= 2) current.push(current[0], current[1]);
      continue;
    }
    for (let n = 0; n < count && i + 1 < commands.length; n++) {
      x += zigzag(commands[i++]);
      y += zigzag(commands[i++]);
      if (cmd === 1) {
        current = [];
        parts.push(current);
      }
      current?.push(x, y);
    }
  }
  return parts;
}

type RawFeature = { type: number; tags: number[]; geometry: number[] };

function readFeature(bytes: Uint8Array): RawFeature {
  const r = new Reader(bytes);
  const f: RawFeature = { type: 0, tags: [], geometry: [] };
  while (!r.done) {
    const tag = r.varint();
    const field = tag >> 3;
    if (field === 2) f.tags = packed(r.bytes());
    else if (field === 3) f.type = r.varint();
    else if (field === 4) f.geometry = packed(r.bytes());
    else r.skip(tag & 7);
  }
  return f;
}

function readLayer(bytes: Uint8Array, wanted?: Set<string>): Layer | null {
  const r = new Reader(bytes);
  let name = '';
  let extent = 4096;
  const keys: string[] = [];
  const values: (string | number | boolean)[] = [];
  const raw: Uint8Array[] = [];
  while (!r.done) {
    const tag = r.varint();
    const field = tag >> 3;
    if (field === 1) {
      name = text.decode(r.bytes());
      if (wanted && !wanted.has(name)) return null;
    } else if (field === 2) raw.push(r.bytes());
    else if (field === 3) keys.push(text.decode(r.bytes()));
    else if (field === 4) values.push(readValue(r.bytes()));
    else if (field === 5) extent = r.varint();
    else r.skip(tag & 7);
  }
  const features: Feature[] = [];
  for (const bytes of raw) {
    const f = readFeature(bytes);
    if (f.type < 1 || f.type > 3) continue;
    const props: Props = {};
    for (let i = 0; i + 1 < f.tags.length; i += 2) props[keys[f.tags[i]]] = values[f.tags[i + 1]];
    features.push({ type: f.type as 1 | 2 | 3, props, parts: decodeGeometry(f.geometry) });
  }
  return { name, extent, features };
}

/** Reads the layers of one tile, optionally only the ones named. */
export function decodeTile(buf: Uint8Array, only?: string[]): Map<string, Layer> {
  const wanted = only ? new Set(only) : undefined;
  const r = new Reader(buf);
  const layers = new Map<string, Layer>();
  while (!r.done) {
    const tag = r.varint();
    if (tag >> 3 === 3) {
      const layer = readLayer(r.bytes(), wanted);
      if (layer) layers.set(layer.name, layer);
    } else r.skip(tag & 7);
  }
  return layers;
}

/** Shoelace area in tile units; positive for an outer ring (clockwise with y down). */
export function ringArea(ring: Ring): number {
  let sum = 0;
  for (let i = 0; i + 3 < ring.length; i += 2) sum += ring[i] * ring[i + 3] - ring[i + 2] * ring[i + 1];
  return sum / 2;
}

/** Groups polygon rings into outer rings with their holes. */
export function polygons(parts: Ring[]): { outer: Ring; holes: Ring[] }[] {
  const out: { outer: Ring; holes: Ring[] }[] = [];
  for (const ring of parts) {
    const area = ringArea(ring);
    if (area > 0) out.push({ outer: ring, holes: [] });
    else if (area < 0 && out.length) out[out.length - 1].holes.push(ring);
  }
  return out;
}
