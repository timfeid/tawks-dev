/**
 * Turns one decoded vector tile into flat typed arrays the renderer uploads
 * as is: a ground grid that follows the terrain, road ribbons draped over it
 * with centre lines, buildings standing on it in chunks, lamp and tree spots,
 * and the road and wall segments driving needs. Pure, so it runs in the
 * worker and in tests. The ground's colours are painted separately, in
 * groundpaint.ts, since painting needs a canvas.
 */

import { ShapeUtils, Vector2 } from 'three';
import type { Frame } from './geo';
import { polygons, type Layer, type Ring } from './mvt';
import { buildGrid, closestOnSegment, type Grid } from './roads';
import {
  DEFAULT_HEIGHT,
  FILLER_ROOF,
  FILLER_WALL,
  HOUSE_AREA,
  LINE,
  ROADS,
  TALL,
  UNKNOWN_HEIGHT,
  facadeColor,
  roadWidth,
  shingleColor,
  sidingColor,
} from './style';
import { flatHeights, heightAt, type Heights } from './terrain';

export type FlatMesh = { pos: Float32Array; col: Uint8Array; idx: Uint32Array };
export type GroundMesh = { pos: Float32Array; nor: Float32Array; uv: Float32Array; idx: Uint32Array };
export type BuildingChunk = FlatMesh & {
  nor: Float32Array;
  uv: Float32Array;
  /** Indices of buildings at least TALL meters high come first; this many of them. */
  tallCount: number;
  cx: number;
  cz: number;
  radius: number;
};

export type TileMesh = {
  x: number;
  y: number;
  ground: GroundMesh;
  /** The tile's heights, for driving on what is drawn. */
  heights: Heights;
  roads: FlatMesh;
  buildings: BuildingChunk[];
  /** Houses made up where OSM maps none, drawn only when the player wants them. */
  fillers: BuildingChunk[];
  /** x, y, z, heading for each street lamp. */
  lamps: Float32Array;
  /** x, y, z, size for each tree. */
  trees: Float32Array;
  /** Drivable road centre segments: x1, z1, x2, z2. */
  segs: Float32Array;
  /** Half width of each segment's road, in meters. */
  segHalf: Float32Array;
  /** Deck heights at both ends of bridge segments: y1, y2, or NaN off bridges. */
  segDeck: Float32Array;
  names: string[];
  /** Named road segments: x1, z1, x2, z2, with nameOf[i] into names. */
  nameSegs: Float32Array;
  nameOf: Uint16Array;
  /** Building outline edges cars bounce off: x1, z1, x2, z2. */
  walls: Float32Array;
  /** The same for filler houses. */
  fillerWalls: Float32Array;
  /** West and north edges and size of the tile, in meters. */
  bounds: [number, number, number];
  /** Lookup grids over segs, nameSegs, walls and fillerWalls. */
  grids: [Grid, Grid, Grid, Grid];
  stats: { buildings: number; houses: number; fillers: number; roads: number; triangles: number };
};

export type BuildOptions = { fillers?: boolean };

export const CHUNKS = 4;
/** The window texture repeats every eight windows of four meters, and every eight floors. */
export const WINDOW_TILE_U = 32;
export const WINDOW_TILE_V = 28;
/** Roads float this far over the ground, and their markings a little more. */
const ROAD_LIFT = 0.18;
const LINE_LIFT = 0.24;
/** Ribbons bend with the ground at least this often, in meters. */
const DRAPE_EVERY = 9;
const MAX_LAMPS = 600;
const MAX_TREES = 500;
const LAMP_EVERY = 38;
const FILLER_SPACING = 23;
const MAX_FILLERS = 2500;

/** Vertex colours are read as linear, so sRGB bytes are converted once here. */
const LINEAR = Array.from({ length: 256 }, (_, i) => {
  const c = i / 255;
  return Math.round((c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4) * 255);
});

type Pt = [number, number];
type Height = (x: number, z: number) => number;

class Mesh {
  pos: number[] = [];
  col: number[] = [];
  idx: number[] = [];

  vertex(x: number, y: number, z: number, c: readonly number[]) {
    this.pos.push(x, y, z);
    this.col.push(LINEAR[Math.round(c[0])], LINEAR[Math.round(c[1])], LINEAR[Math.round(c[2])]);
    return this.pos.length / 3 - 1;
  }

  /** Adds a triangle wound to face up. */
  up(into: number[], a: number, b: number, c: number) {
    const p = this.pos;
    const ax = p[a * 3], az = p[a * 3 + 2];
    const cross = (p[b * 3 + 2] - az) * (p[c * 3] - ax) - (p[b * 3] - ax) * (p[c * 3 + 2] - az);
    if (cross >= 0) into.push(a, b, c);
    else into.push(a, c, b);
  }

  done(): FlatMesh {
    return { pos: new Float32Array(this.pos), col: new Uint8Array(this.col), idx: new Uint32Array(this.idx) };
  }
}

class BuildingMesh extends Mesh {
  nor: number[] = [];
  uv: number[] = [];
  tall: number[] = [];
  minX = Infinity;
  maxX = -Infinity;
  minZ = Infinity;
  maxZ = -Infinity;

  v(x: number, y: number, z: number, nx: number, ny: number, nz: number, u: number, w: number, c: readonly number[]) {
    this.nor.push(nx, ny, nz);
    this.uv.push(u, w);
    if (x < this.minX) this.minX = x;
    if (x > this.maxX) this.maxX = x;
    if (z < this.minZ) this.minZ = z;
    if (z > this.maxZ) this.maxZ = z;
    return this.vertex(x, y, z, c);
  }

  /** A flat polygon (three or four corners) with its own face normal, turned to face away from `centre`. */
  face(corners: [number, number, number][], u: number[], w: number[], c: readonly number[], into: number[], centre: [number, number, number]) {
    let pts = corners;
    const [a, b, d] = pts;
    const ux = b[0] - a[0], uy = b[1] - a[1], uz = b[2] - a[2];
    const vx = d[0] - a[0], vy = d[1] - a[1], vz = d[2] - a[2];
    let nx = uy * vz - uz * vy;
    let ny = uz * vx - ux * vz;
    let nz = ux * vy - uy * vx;
    const len = Math.hypot(nx, ny, nz) || 1;
    nx /= len;
    ny /= len;
    nz /= len;
    if (nx * (a[0] - centre[0]) + ny * (a[1] - centre[1]) + nz * (a[2] - centre[2]) < 0) {
      pts = [...pts].reverse();
      u = [...u].reverse();
      w = [...w].reverse();
      nx = -nx;
      ny = -ny;
      nz = -nz;
    }
    const base = this.pos.length / 3;
    pts.forEach((p, i) => this.v(p[0], p[1], p[2], nx, ny, nz, u[i], w[i], c));
    for (let i = 1; i + 1 < pts.length; i++) into.push(base, base + i, base + i + 1);
  }

  get empty() {
    return this.pos.length === 0;
  }

  chunk(): BuildingChunk {
    const tallCount = this.tall.length;
    const idx = new Uint32Array(tallCount + this.idx.length);
    idx.set(this.tall);
    idx.set(this.idx, tallCount);
    return {
      pos: new Float32Array(this.pos),
      col: new Uint8Array(this.col),
      idx,
      nor: new Float32Array(this.nor),
      uv: new Float32Array(this.uv),
      tallCount,
      cx: (this.minX + this.maxX) / 2,
      cz: (this.minZ + this.maxZ) / 2,
      radius: Math.hypot(this.maxX - this.minX, this.maxZ - this.minZ) / 2,
    };
  }
}

function str(v: unknown): string {
  return typeof v === 'string' ? v : '';
}

function num(v: unknown, fallback: number): number {
  return typeof v === 'number' && Number.isFinite(v) ? v : fallback;
}

/** A tiny stable hash for scattering things the same way on every machine. */
export function hash2(x: number, y: number): number {
  let h = Math.imul(Math.floor(x) | 0, 374761393) ^ Math.imul(Math.floor(y) | 0, 668265263);
  h = Math.imul(h ^ (h >>> 13), 1274126177);
  return ((h ^ (h >>> 16)) >>> 0) / 4294967296;
}

function toWorld(frame: Frame, tx: number, ty: number, extent: number, ring: Ring): Pt[] {
  const out: Pt[] = [];
  for (let i = 0; i + 1 < ring.length; i += 2) {
    const p = frame.fromTile(tx, ty, extent, ring[i], ring[i + 1]);
    const last = out[out.length - 1];
    if (!last || Math.abs(last[0] - p[0]) > 0.01 || Math.abs(last[1] - p[1]) > 0.01) out.push(p);
  }
  if (out.length > 2) {
    const a = out[0];
    const b = out[out.length - 1];
    if (Math.abs(a[0] - b[0]) < 0.01 && Math.abs(a[1] - b[1]) < 0.01) out.pop();
  }
  return out;
}

function pointInRing(x: number, z: number, ring: Pt[]): boolean {
  let inside = false;
  for (let i = 0, j = ring.length - 1; i < ring.length; j = i++) {
    const [xi, zi] = ring[i];
    const [xj, zj] = ring[j];
    if (zi > z !== zj > z && x < ((xj - xi) * (z - zi)) / (zj - zi) + xi) inside = !inside;
  }
  return inside;
}

function ringArea(ring: Pt[]): number {
  let area = 0;
  for (let i = 0; i < ring.length; i++) {
    const [x1, z1] = ring[i];
    const [x2, z2] = ring[(i + 1) % ring.length];
    area += x1 * z2 - x2 * z1;
  }
  return area / 2;
}

/** Makes a ring wind so wall normals face outward: positive area for outer rings, negative for holes. */
function outward(ring: Pt[], wantPositive: boolean): Pt[] {
  return ringArea(ring) > 0 === wantPositive ? ring : ring.slice().reverse();
}

/** Adds points so no stretch of the line is longer than `every`, for following the ground. */
export function subdivide(line: Pt[], every: number): Pt[] {
  const out: Pt[] = [line[0]];
  for (let i = 0; i + 1 < line.length; i++) {
    const [x1, z1] = line[i];
    const [x2, z2] = line[i + 1];
    const steps = Math.max(1, Math.ceil(Math.hypot(x2 - x1, z2 - z1) / every));
    for (let s = 1; s <= steps; s++) out.push([x1 + ((x2 - x1) * s) / steps, z1 + ((z2 - z1) * s) / steps]);
  }
  return out;
}

/** A ribbon along a polyline, mitred at bends, its edges set on the ground, capped with a disc at each end. */
export function ribbon(mesh: Mesh, line: Pt[], half: number, y: Height, color: readonly number[], caps: boolean | [boolean, boolean] = true) {
  const n = line.length;
  if (n < 2) return 0;
  let tris = 0;
  const base = mesh.pos.length / 3;
  for (let i = 0; i < n; i++) {
    const prev = line[Math.max(0, i - 1)];
    const next = line[Math.min(n - 1, i + 1)];
    let dx = next[0] - prev[0];
    let dz = next[1] - prev[1];
    const len = Math.hypot(dx, dz) || 1;
    dx /= len;
    dz /= len;
    let scale = 1;
    if (i > 0 && i < n - 1) {
      const ax = line[i][0] - prev[0];
      const az = line[i][1] - prev[1];
      const al = Math.hypot(ax, az) || 1;
      const cos = (ax / al) * dx + (az / al) * dz;
      scale = 1 / Math.max(0.5, Math.sqrt((1 + cos) / 2));
    }
    const nx = -dz * half * scale;
    const nz = dx * half * scale;
    const [lx, lz] = [line[i][0] + nx, line[i][1] + nz];
    const [rx, rz] = [line[i][0] - nx, line[i][1] - nz];
    mesh.vertex(lx, y(lx, lz), lz, color);
    mesh.vertex(rx, y(rx, rz), rz, color);
  }
  for (let i = 0; i < n - 1; i++) {
    const a = base + i * 2;
    mesh.up(mesh.idx, a, a + 2, a + 1);
    mesh.up(mesh.idx, a + 1, a + 2, a + 3);
    tris += 2;
  }
  const [capStart, capEnd] = Array.isArray(caps) ? caps : [caps, caps];
  if (capStart) tris += disc(mesh, line[0][0], line[0][1], half, y, color);
  if (capEnd) tris += disc(mesh, line[n - 1][0], line[n - 1][1], half, y, color);
  return tris;
}

function disc(mesh: Mesh, x: number, z: number, r: number, y: Height, color: readonly number[]) {
  const sides = 8;
  const centre = mesh.vertex(x, y(x, z), z, color);
  for (let i = 0; i < sides; i++) {
    const a = (i / sides) * Math.PI * 2;
    const px = x + Math.cos(a) * r;
    const pz = z + Math.sin(a) * r;
    mesh.vertex(px, y(px, pz), pz, color);
  }
  for (let i = 0; i < sides; i++) mesh.up(mesh.idx, centre, centre + 1 + ((i + 1) % sides), centre + 1 + i);
  return sides;
}

function dashes(mesh: Mesh, line: Pt[], y: Height) {
  let tris = 0;
  const dash = 3;
  const gap = 6;
  let carry = 0;
  for (let i = 0; i + 1 < line.length; i++) {
    const [x1, z1] = line[i];
    const [x2, z2] = line[i + 1];
    const len = Math.hypot(x2 - x1, z2 - z1);
    let t = carry;
    while (t < len) {
      const end = Math.min(len, t + dash);
      const a: Pt = [x1 + ((x2 - x1) * t) / len, z1 + ((z2 - z1) * t) / len];
      const b: Pt = [x1 + ((x2 - x1) * end) / len, z1 + ((z2 - z1) * end) / len];
      tris += ribbon(mesh, [a, b], 0.12, y, LINE, false);
      t += dash + gap;
    }
    carry = t - len;
  }
  return tris;
}

/** Walls of a ring from `low` to `high`, windows mapped from `floor`. */
function walls(mesh: BuildingMesh, ring: Pt[], low: number, high: number, floor: number, color: readonly number[], into: number[]) {
  let along = 0;
  let tris = 0;
  for (let i = 0; i < ring.length; i++) {
    const [x1, z1] = ring[i];
    const [x2, z2] = ring[(i + 1) % ring.length];
    const len = Math.hypot(x2 - x1, z2 - z1);
    if (len < 0.05) continue;
    const nx = (z2 - z1) / len;
    const nz = -(x2 - x1) / len;
    const u1 = along / WINDOW_TILE_U;
    const u2 = (along + len) / WINDOW_TILE_U;
    const v1 = (low - floor) / WINDOW_TILE_V;
    const v2 = (high - floor) / WINDOW_TILE_V;
    const a = mesh.v(x1, low, z1, nx, 0, nz, u1, v1, color);
    mesh.v(x2, low, z2, nx, 0, nz, u2, v1, color);
    mesh.v(x2, high, z2, nx, 0, nz, u2, v2, color);
    mesh.v(x1, high, z1, nx, 0, nz, u1, v2, color);
    into.push(a, a + 2, a + 1, a, a + 3, a + 2);
    tris += 2;
    along += len;
  }
  return tris;
}

/** A building of any shape with a flat roof. */
function extrude(mesh: BuildingMesh, outer: Pt[], holes: Pt[][], floor: number, low: number, high: number, color: readonly number[], roofColor: readonly number[]) {
  const into = high - floor >= TALL ? mesh.tall : mesh.idx;
  let tris = walls(mesh, outer, low, high, floor, color, into);
  for (const h of holes) tris += walls(mesh, h, low, high, floor, color, into);
  const contour = outer.map(([x, z]) => new Vector2(x, z));
  const holeVs = holes.map((h) => h.map(([x, z]) => new Vector2(x, z)));
  const base = mesh.pos.length / 3;
  for (const [x, z] of outer) mesh.v(x, high, z, 0, 1, 0, 0.005, 0.005, roofColor);
  for (const h of holes) for (const [x, z] of h) mesh.v(x, high, z, 0, 1, 0, 0.005, 0.005, roofColor);
  for (const [a, b, c] of ShapeUtils.triangulateShape(contour, holeVs)) {
    mesh.up(into, base + a, base + b, base + c);
    tris++;
  }
  return tris;
}

/** Four corners close enough to a rectangle to carry a gable roof, in ring order. */
export function rectangular(ring: Pt[]): boolean {
  if (ring.length !== 4) return false;
  for (let i = 0; i < 4; i++) {
    const [ax, az] = ring[(i + 3) % 4];
    const [bx, bz] = ring[i];
    const [cx, cz] = ring[(i + 1) % 4];
    const ux = ax - bx, uz = az - bz, vx = cx - bx, vz = cz - bz;
    const cos = (ux * vx + uz * vz) / (Math.hypot(ux, uz) * Math.hypot(vx, vz) || 1);
    if (Math.abs(cos) > 0.26) return false;
  }
  return true;
}

/** A house: walls to the eaves and a gable roof whose ridge runs along the longer side. */
function house(mesh: BuildingMesh, ring: Pt[], floor: number, eaves: number, rise: number, wall: readonly number[], roof: readonly number[]) {
  const into = mesh.idx;
  let tris = walls(mesh, ring, floor - 0.5, eaves, floor, wall, into);
  const long = Math.hypot(ring[1][0] - ring[0][0], ring[1][1] - ring[0][1]) >= Math.hypot(ring[2][0] - ring[1][0], ring[2][1] - ring[1][1]);
  const [a, b, c, d] = long ? ring : [ring[1], ring[2], ring[3], ring[0]];
  const mid = (p: Pt, q: Pt): Pt => [(p[0] + q[0]) / 2, (p[1] + q[1]) / 2];
  const m1 = mid(d, a);
  const m2 = mid(b, c);
  const top = eaves + rise;
  const P = (p: Pt, y: number): [number, number, number] => [p[0], y, p[1]];
  const flat = [0.005, 0.005, 0.005, 0.005];
  const centre: [number, number, number] = [(a[0] + c[0]) / 2, eaves, (a[1] + c[1]) / 2];
  mesh.face([P(a, eaves), P(m1, top), P(m2, top), P(b, eaves)], flat, flat, roof, into, centre);
  mesh.face([P(c, eaves), P(m2, top), P(m1, top), P(d, eaves)], flat, flat, roof, into, centre);
  mesh.face([P(b, eaves), P(m2, top), P(c, eaves)], [0, 0, 0], [0, 0, 0], wall, into, centre);
  mesh.face([P(d, eaves), P(m1, top), P(a, eaves)], [0, 0, 0], [0, 0, 0], wall, into, centre);
  return tris + 6;
}

/**
 * The pieces of a line inside a box. Tiles carry a margin of their
 * neighbours' features, and heights there are only a guess, so each tile
 * draws just its own part. Flags say which ends are the line's real ends.
 */
export function clipLine(line: Pt[], x0: number, z0: number, x1: number, z1: number): { line: Pt[]; startCap: boolean; endCap: boolean }[] {
  const out: { line: Pt[]; startCap: boolean; endCap: boolean }[] = [];
  let current: Pt[] | null = null;
  let startCap = false;
  for (let i = 0; i + 1 < line.length; i++) {
    const [ax, az] = line[i];
    const [bx, bz] = line[i + 1];
    let t0 = 0;
    let t1 = 1;
    const dx = bx - ax;
    const dz = bz - az;
    let inside = true;
    for (const [p, q] of [
      [-dx, ax - x0],
      [dx, x1 - ax],
      [-dz, az - z0],
      [dz, z1 - az],
    ]) {
      if (p === 0) {
        if (q < 0) inside = false;
      } else {
        const r = q / p;
        if (p < 0) t0 = Math.max(t0, r);
        else t1 = Math.min(t1, r);
      }
    }
    if (!inside || t0 > t1) {
      if (current) out.push({ line: current, startCap, endCap: false });
      current = null;
      continue;
    }
    const a: Pt = [ax + dx * t0, az + dz * t0];
    const b: Pt = [ax + dx * t1, az + dz * t1];
    if (!current) {
      current = [a];
      startCap = i === 0 && t0 === 0;
    }
    current.push(b);
    if (t1 < 1) {
      out.push({ line: current, startCap, endCap: false });
      current = null;
    }
  }
  if (current) out.push({ line: current, startCap, endCap: true });
  return out.filter((piece) => piece.line.length >= 2);
}

function lerpDeck(line: Pt[], start: number, end: number): number[] {
  const lengths = [0];
  for (let i = 1; i < line.length; i++) lengths.push(lengths[i - 1] + Math.hypot(line[i][0] - line[i - 1][0], line[i][1] - line[i - 1][1]));
  const total = lengths[lengths.length - 1] || 1;
  return lengths.map((l) => start + ((end - start) * l) / total);
}

export function buildTile(layers: Map<string, Layer>, tx: number, ty: number, frame: Frame, terrain?: Heights | null, opts: BuildOptions = {}): TileMesh {
  const [minX, minZ] = frame.fromTile(tx, ty, 1, 0, 0);
  const size = frame.metersPerTile;
  const bounds: [number, number, number] = [minX, minZ, size];
  const heights = terrain ?? flatHeights(bounds);
  const ground: Height = (x, z) => heightAt(heights, x, z);
  const roads = new Mesh();
  const lines = new Mesh();
  const chunks: BuildingMesh[] = Array.from({ length: CHUNKS * CHUNKS }, () => new BuildingMesh());
  const fillerChunks: BuildingMesh[] = Array.from({ length: CHUNKS * CHUNKS }, () => new BuildingMesh());
  const chunkAt = (list: BuildingMesh[], x: number, z: number) => {
    const cx = Math.min(CHUNKS - 1, Math.max(0, Math.floor(((x - minX) / size) * CHUNKS)));
    const cz = Math.min(CHUNKS - 1, Math.max(0, Math.floor(((z - minZ) / size) * CHUNKS)));
    return list[cz * CHUNKS + cx];
  };
  const lamps: number[] = [];
  const trees: number[] = [];
  const segs: number[] = [];
  const segHalf: number[] = [];
  const segDeck: number[] = [];
  const names: string[] = [];
  const nameIndex = new Map<string, number>();
  const nameSegs: number[] = [];
  const nameOf: number[] = [];
  const wallSegs: number[] = [];
  const fillerWalls: number[] = [];
  const stats = { buildings: 0, houses: 0, fillers: 0, roads: 0, triangles: 0 };

  const scatterTrees = (ring: Pt[], spacing: number) => {
    let x0 = Infinity, x1 = -Infinity, z0 = Infinity, z1 = -Infinity;
    for (const [x, z] of ring) {
      x0 = Math.min(x0, x);
      x1 = Math.max(x1, x);
      z0 = Math.min(z0, z);
      z1 = Math.max(z1, z);
    }
    for (let gx = Math.ceil(x0 / spacing); gx * spacing < x1; gx++) {
      for (let gz = Math.ceil(z0 / spacing); gz * spacing < z1; gz++) {
        if (trees.length / 4 >= MAX_TREES) return;
        const x = gx * spacing + (hash2(gx, gz) - 0.5) * spacing * 0.8;
        const z = gz * spacing + (hash2(gz, gx + 7) - 0.5) * spacing * 0.8;
        const own = x >= minX && z >= minZ && x < minX + size && z < minZ + size;
        if (own && pointInRing(x, z, ring)) trees.push(x, ground(x, z), z, 0.8 + hash2(gx + 3, gz) * 0.6);
      }
    }
  };
  const treeLayers: [string, (p: Record<string, unknown>) => number][] = [
    ['park', () => 22],
    ['landcover', (p) => (['wood', 'forest'].includes(str(p.class)) ? 14 : 0)],
  ];
  for (const [name, spacing] of treeLayers) {
    const layer = layers.get(name);
    if (!layer) continue;
    for (const f of layer.features) {
      const every = spacing(f.props);
      if (f.type !== 3 || !every) continue;
      for (const poly of polygons(f.parts)) scatterTrees(toWorld(frame, tx, ty, layer.extent, poly.outer), every);
    }
  }

  // Made up houses keep out of anything mapped as something other than homes.
  const keepOut: Pt[][] = [];
  if (opts.fillers) {
    const homes = new Set(['residential', 'suburb', 'quarter', 'neighbourhood']);
    const open = new Set(['wood', 'forest', 'farmland', 'wetland', 'sand', 'rock', 'ice', 'grass']);
    const sources: [string, (p: Record<string, unknown>) => boolean][] = [
      ['landuse', (p) => !homes.has(str(p.class))],
      ['landcover', (p) => open.has(str(p.class)) || open.has(str(p.subclass))],
      ['park', () => true],
      ['water', () => true],
    ];
    for (const [name, excluded] of sources) {
      const layer = layers.get(name);
      if (!layer) continue;
      for (const f of layer.features) {
        if (f.type !== 3 || !excluded(f.props)) continue;
        for (const poly of polygons(f.parts)) keepOut.push(toWorld(frame, tx, ty, layer.extent, poly.outer));
      }
    }
  }
  const minorRoads: { line: Pt[]; half: number }[] = [];

  const transportation = layers.get('transportation');
  if (transportation) {
    const sorted = transportation.features
      .filter((f) => f.type === 2 && ROADS[str(f.props.class)] && str(f.props.brunnel) !== 'tunnel')
      .sort((a, b) => ROADS[str(a.props.class)].rank - ROADS[str(b.props.class)].rank);
    for (const f of sorted) {
      const cls = str(f.props.class);
      const style = ROADS[cls];
      const half = roadWidth(cls, num(f.props.ramp, 0) === 1) / 2;
      const lift = ROAD_LIFT + style.rank * 0.01;
      const bridge = str(f.props.brunnel) === 'bridge';
      for (const part of f.parts) {
        const raw = toWorld(frame, tx, ty, transportation.extent, part);
        if (raw.length < 2) continue;
        stats.roads++;
        const whole = subdivide(raw, DRAPE_EVERY);
        let y: Height = (x, z) => ground(x, z) + lift;
        let deck: number[] | null = null;
        if (bridge) {
          // A bridge runs straight between the ground at its two ends, over whatever lies between.
          const line = whole;
          const start = ground(line[0][0], line[0][1]);
          const end = ground(line[line.length - 1][0], line[line.length - 1][1]);
          deck = lerpDeck(line, start, end).map((d, i) => Math.max(d, ground(line[i][0], line[i][1]) + 0.5));
          const at = (x: number, z: number) => {
            let best = 0;
            let bestD = Infinity;
            for (let i = 0; i + 1 < line.length; i++) {
              const [px, pz, t] = closestOnSegment(x, z, line[i][0], line[i][1], line[i + 1][0], line[i + 1][1]);
              const d = Math.hypot(px - x, pz - z);
              if (d < bestD) {
                bestD = d;
                best = deck![i] + (deck![i + 1] - deck![i]) * t;
              }
            }
            return best;
          };
          y = (x, z) => at(x, z) + lift;
        }
        for (const piece of clipLine(whole, minX, minZ, minX + size, minZ + size)) {
          stats.triangles += ribbon(roads, piece.line, half, y, style.color, [piece.startCap, piece.endCap]);
          if (style.line && num(f.props.oneway, 0) === 0) stats.triangles += dashes(lines, piece.line, (x, z) => y(x, z) + (LINE_LIFT - ROAD_LIFT));
        }
        for (let i = 0; i + 1 < raw.length; i++) {
          segs.push(raw[i][0], raw[i][1], raw[i + 1][0], raw[i + 1][1]);
          segHalf.push(half);
          if (bridge) segDeck.push(y(raw[i][0], raw[i][1]) - lift, y(raw[i + 1][0], raw[i + 1][1]) - lift);
          else segDeck.push(NaN, NaN);
        }
        if (style.lamps && !bridge) placeLamps(raw, half);
        if (cls === 'minor' && !bridge) minorRoads.push({ line: raw, half });
      }
    }
  }

  function placeLamps(line: Pt[], half: number) {
    let carry = LAMP_EVERY / 2;
    let side = 1;
    for (let i = 0; i + 1 < line.length; i++) {
      const [x1, z1] = line[i];
      const [x2, z2] = line[i + 1];
      const len = Math.hypot(x2 - x1, z2 - z1);
      if (len < 0.01) continue;
      const dx = (x2 - x1) / len;
      const dz = (z2 - z1) / len;
      let t = carry;
      while (t < len) {
        if (lamps.length / 4 >= MAX_LAMPS) return;
        const off = (half + 0.8) * side;
        const x = x1 + dx * t - dz * off;
        const z = z1 + dz * t + dx * off;
        if (x >= minX && z >= minZ && x < minX + size && z < minZ + size) lamps.push(x, ground(x, z), z, Math.atan2(dz * side, -dx * side));
        side = -side;
        t += LAMP_EVERY;
      }
      carry = t - len;
    }
  }

  const named = layers.get('transportation_name');
  if (named) {
    for (const f of named.features) {
      if (f.type !== 2) continue;
      const name = str(f.props.name_en) || str(f.props.name) || str(f.props.ref);
      if (!name) continue;
      let i = nameIndex.get(name);
      if (i === undefined) {
        i = names.length;
        names.push(name);
        nameIndex.set(name, i);
      }
      for (const part of f.parts) {
        const line = toWorld(frame, tx, ty, named.extent, part);
        for (let k = 0; k + 1 < line.length; k++) {
          nameSegs.push(line[k][0], line[k][1], line[k + 1][0], line[k + 1][1]);
          nameOf.push(i);
        }
      }
    }
  }

  // Ten meter cells that hold a mapped building, so made up houses keep clear of real ones.
  const CELL = 10;
  const cells = Math.ceil(size / CELL) + 2;
  const taken = new Uint8Array(cells * cells);
  const mark = (x: number, z: number, r: number) => {
    const ax = Math.floor((x - r - minX) / CELL), bx = Math.floor((x + r - minX) / CELL);
    const az = Math.floor((z - r - minZ) / CELL), bz = Math.floor((z + r - minZ) / CELL);
    for (let cz = Math.max(0, az); cz <= Math.min(cells - 1, bz); cz++) for (let cx = Math.max(0, ax); cx <= Math.min(cells - 1, bx); cx++) taken[cz * cells + cx] = 1;
  };
  const free = (x: number, z: number, r: number) => {
    const ax = Math.floor((x - r - minX) / CELL), bx = Math.floor((x + r - minX) / CELL);
    const az = Math.floor((z - r - minZ) / CELL), bz = Math.floor((z + r - minZ) / CELL);
    if (ax < 0 || az < 0 || bx >= cells || bz >= cells) return false;
    for (let cz = az; cz <= bz; cz++) for (let cx = ax; cx <= bx; cx++) if (taken[cz * cells + cx]) return false;
    return true;
  };

  type Footprint = { outer: Pt[]; holes: Pt[][]; wx: number; wz: number; floor: number; area: number; mapped: number; min: number; colour: string | number | boolean | undefined; inside: boolean };
  const footprints: Footprint[] = [];
  const building = layers.get('building');
  if (building) {
    const extent = building.extent;
    for (const f of building.features) {
      if (f.type !== 3 || f.props.hide_3d === true) continue;
      const mapped = num(f.props.render_height, DEFAULT_HEIGHT);
      const min = Math.max(0, num(f.props.render_min_height, 0));
      for (const poly of polygons(f.parts)) {
        let cx = 0;
        let cy = 0;
        const pts = poly.outer.length / 2;
        for (let i = 0; i < poly.outer.length; i += 2) {
          cx += poly.outer[i];
          cy += poly.outer[i + 1];
        }
        cx /= pts;
        cy /= pts;
        const outer = outward(toWorld(frame, tx, ty, extent, poly.outer), true);
        if (outer.length < 3) continue;
        let wx = 0, wz = 0, floor = Infinity, far = 0;
        for (const [x, z] of outer) {
          wx += x / outer.length;
          wz += z / outer.length;
          floor = Math.min(floor, ground(x, z));
        }
        for (const [x, z] of outer) far = Math.max(far, Math.hypot(x - wx, z - wz));
        mark(wx, wz, far);
        // A building is drawn by the tile its middle falls in, so neighbours never draw it twice.
        const inside = cx >= 0 && cy >= 0 && cx < extent && cy < extent;
        const holes = inside ? poly.holes.map((h) => outward(toWorld(frame, tx, ty, extent, h), false)).filter((h) => h.length >= 3) : [];
        footprints.push({ outer, holes, wx, wz, floor, area: ringArea(outer), mapped, min, colour: f.props.colour, inside });
      }
    }
  }

  // Rowhouses stand wall to wall and get flat roofs; detached houses get gables.
  const NEAR = 8;
  const near = new Map<number, Footprint[]>();
  const nearKey = (x: number, z: number) => Math.floor((x - minX) / NEAR) * 4096 + Math.floor((z - minZ) / NEAR);
  for (const fp of footprints) {
    const k = nearKey(fp.wx, fp.wz);
    const list = near.get(k);
    if (list) list.push(fp);
    else near.set(k, [fp]);
  }
  const attached = (fp: Footprint) => {
    const gx = Math.floor((fp.wx - minX) / NEAR);
    const gz = Math.floor((fp.wz - minZ) / NEAR);
    for (let dx = -1; dx <= 1; dx++) {
      for (let dz = -1; dz <= 1; dz++) {
        for (const other of near.get((gx + dx) * 4096 + gz + dz) ?? []) {
          if (other !== fp && Math.hypot(other.wx - fp.wx, other.wz - fp.wz) < 8.5) return true;
        }
      }
    }
    return false;
  };

  for (const fp of footprints) {
    if (!fp.inside) continue;
    const { outer, holes, wx, wz, floor, area, mapped, min } = fp;
    const seed = hash2(outer[0][0], outer[0][1]) * 1000;
    const chunk = chunkAt(chunks, wx, wz);
    const unknown = mapped === UNKNOWN_HEIGHT && min === 0;
    if (unknown && area < HOUSE_AREA && !holes.length && !attached(fp)) {
      const eaves = floor + 5 + hash2(seed, 3) * 1.6;
      const wall = sidingColor(seed);
      const roof = shingleColor(seed * 7);
      if (rectangular(outer)) stats.triangles += house(chunk, outer, floor, eaves, 2 + hash2(seed, 9) * 1.4, wall, roof);
      else stats.triangles += extrude(chunk, outer, holes, floor, floor - 0.5, eaves + 1, wall, roof);
      stats.houses++;
    } else {
      // Unmapped rowhouses and small blocks: three or four storeys, flat roofed.
      const height = unknown ? (area < HOUSE_AREA ? 9.5 + hash2(seed, 5) * 3 : 7 + hash2(seed, 5) * 2) : Math.max(3, mapped);
      if (height - min < 1) continue;
      const color = unknown && area < HOUSE_AREA ? sidingColor(seed + 3) : facadeColor(seed, fp.colour);
      const roof = color.map((c) => Math.min(255, c * 0.82 + 20));
      stats.triangles += extrude(chunk, outer, holes, floor, floor + min - (min === 0 ? 0.5 : 0), floor + height, color, roof);
    }
    stats.buildings++;
    if (min < 2.5) {
      for (let i = 0; i < outer.length; i++) {
        const a = outer[i];
        const b = outer[(i + 1) % outer.length];
        wallSegs.push(a[0], a[1], b[0], b[1]);
      }
    }
  }

  if (opts.fillers) {
    const roadCells = new Uint8Array(cells * cells);
    for (const { line, half } of minorRoads) {
      for (const [x, z] of subdivide(line, 4)) {
        const r = half + 3;
        const ax = Math.floor((x - r - minX) / CELL), bx = Math.floor((x + r - minX) / CELL);
        const az = Math.floor((z - r - minZ) / CELL), bz = Math.floor((z + r - minZ) / CELL);
        for (let cz = Math.max(0, az); cz <= Math.min(cells - 1, bz); cz++) for (let cx = Math.max(0, ax); cx <= Math.min(cells - 1, bx); cx++) roadCells[cz * cells + cx] = 1;
      }
    }
    const boxes = keepOut.map((ring) => {
      let x0 = Infinity, x1 = -Infinity, z0 = Infinity, z1 = -Infinity;
      for (const [x, z] of ring) {
        x0 = Math.min(x0, x);
        x1 = Math.max(x1, x);
        z0 = Math.min(z0, z);
        z1 = Math.max(z1, z);
      }
      return { ring, x0, x1, z0, z1 };
    });
    const keptOut = (x: number, z: number) => boxes.some((b) => x >= b.x0 && x <= b.x1 && z >= b.z0 && z <= b.z1 && pointInRing(x, z, b.ring));
    for (const { line, half } of minorRoads) {
      let carry = FILLER_SPACING / 2;
      for (let i = 0; i + 1 < line.length; i++) {
        const [x1, z1] = line[i];
        const [x2, z2] = line[i + 1];
        const len = Math.hypot(x2 - x1, z2 - z1);
        if (len < 1) continue;
        const dx = (x2 - x1) / len;
        const dz = (z2 - z1) / len;
        let t = carry;
        while (t < len - 4) {
          for (const side of [1, -1]) {
            if (stats.fillers >= MAX_FILLERS) break;
            const setback = half + 12;
            const x = x1 + dx * t - dz * setback * side;
            const z = z1 + dz * t + dx * setback * side;
            if (x < minX || z < minZ || x >= minX + size || z >= minZ + size) continue;
            if (!free(x, z, 7) || keptOut(x, z)) continue;
            const cell = Math.floor((z - minZ) / CELL) * cells + Math.floor((x - minX) / CELL);
            if (roadCells[cell]) continue;
            const w = 4.5 + hash2(x, z) * 1.2;
            const d = 5 + hash2(z, x) * 1.5;
            const ring = outward(
              [
                [x + dx * w - dz * d, z + dz * w + dx * d],
                [x - dx * w - dz * d, z - dz * w + dx * d],
                [x - dx * w + dz * d, z - dz * w - dx * d],
                [x + dx * w + dz * d, z + dz * w - dx * d],
              ],
              true,
            );
            let floor = Infinity;
            for (const [px, pz] of ring) floor = Math.min(floor, ground(px, pz));
            house(chunkAt(fillerChunks, x, z), ring, floor, floor + 5 + hash2(x + 1, z) * 1.2, 2.2, FILLER_WALL, FILLER_ROOF);
            mark(x, z, 8);
            stats.fillers++;
            for (let k = 0; k < 4; k++) fillerWalls.push(ring[k][0], ring[k][1], ring[(k + 1) % 4][0], ring[(k + 1) % 4][1]);
          }
          t += FILLER_SPACING;
        }
        carry = t - len;
      }
    }
  }

  const lineBase = roads.pos.length / 3;
  for (const v of lines.pos) roads.pos.push(v);
  for (const c of lines.col) roads.col.push(c);
  for (const i of lines.idx) roads.idx.push(i + lineBase);

  const segArr = new Float32Array(segs);
  const nameArr = new Float32Array(nameSegs);
  const wallArr = new Float32Array(wallSegs);
  const fillerArr = new Float32Array(fillerWalls);
  return {
    x: tx,
    y: ty,
    ground: groundMesh(heights),
    heights,
    roads: roads.done(),
    buildings: chunks.filter((c) => !c.empty).map((c) => c.chunk()),
    fillers: fillerChunks.filter((c) => !c.empty).map((c) => c.chunk()),
    lamps: new Float32Array(lamps),
    trees: new Float32Array(trees),
    segs: segArr,
    segHalf: new Float32Array(segHalf),
    segDeck: new Float32Array(segDeck),
    names,
    nameSegs: nameArr,
    nameOf: new Uint16Array(nameOf),
    walls: wallArr,
    fillerWalls: fillerArr,
    bounds,
    grids: [
      buildGrid(segArr, minX, minZ, size),
      buildGrid(nameArr, minX, minZ, size),
      buildGrid(wallArr, minX, minZ, size),
      buildGrid(fillerArr, minX, minZ, size),
    ],
    stats,
  };
}

/** The ground grid, two triangles a cell split the way heightAt reads them, with the paint mapped north at v = 0. */
export function groundMesh(t: Heights): GroundMesh {
  const n = t.n;
  const [minX, minZ, size] = t.bounds;
  const cell = size / (n - 1);
  const pos = new Float32Array(n * n * 3);
  const nor = new Float32Array(n * n * 3);
  const uv = new Float32Array(n * n * 2);
  const at = (i: number, j: number) => t.h[Math.min(n - 1, Math.max(0, j)) * n + Math.min(n - 1, Math.max(0, i))];
  for (let j = 0; j < n; j++) {
    for (let i = 0; i < n; i++) {
      const k = j * n + i;
      pos[k * 3] = minX + i * cell;
      pos[k * 3 + 1] = t.h[k];
      pos[k * 3 + 2] = minZ + j * cell;
      const nx = (at(i - 1, j) - at(i + 1, j)) / (2 * cell);
      const nz = (at(i, j - 1) - at(i, j + 1)) / (2 * cell);
      const len = Math.hypot(nx, 1, nz);
      nor[k * 3] = nx / len;
      nor[k * 3 + 1] = 1 / len;
      nor[k * 3 + 2] = nz / len;
      uv[k * 2] = i / (n - 1);
      uv[k * 2 + 1] = j / (n - 1);
    }
  }
  const idx = new Uint32Array((n - 1) * (n - 1) * 6);
  let o = 0;
  for (let j = 0; j + 1 < n; j++) {
    for (let i = 0; i + 1 < n; i++) {
      const a = j * n + i;
      const b = a + 1;
      const c = a + n;
      const d = c + 1;
      idx.set([a, d, b, a, c, d], o);
      o += 6;
    }
  }
  return { pos, nor, uv, idx };
}

/** Every array buffer in a tile mesh, for handing it to the main thread without a copy. */
export function transferables(t: TileMesh): ArrayBuffer[] {
  const out: ArrayBuffer[] = [];
  const add = (a: { buffer: ArrayBufferLike }) => out.push(a.buffer as ArrayBuffer);
  [t.ground.pos, t.ground.nor, t.ground.uv, t.ground.idx, t.heights.h].forEach(add);
  [t.roads.pos, t.roads.col, t.roads.idx].forEach(add);
  for (const c of [...t.buildings, ...t.fillers]) [c.pos, c.col, c.idx, c.nor, c.uv].forEach(add);
  [t.lamps, t.trees, t.segs, t.segHalf, t.segDeck, t.nameSegs, t.nameOf, t.walls, t.fillerWalls].forEach(add);
  for (const g of t.grids) [g.start, g.items].forEach(add);
  return out;
}
