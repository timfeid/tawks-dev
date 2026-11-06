/**
 * Turns one decoded vector tile into flat typed arrays the renderer uploads
 * as is: a ground layer of land use, parks and water, road ribbons with
 * centre lines, extruded buildings in chunks, lamp and tree spots, and the
 * road and wall segments driving needs. Pure, so it runs in the worker and
 * in tests.
 */

import { ShapeUtils, Vector2 } from 'three';
import type { Frame } from './geo';
import { polygons, type Layer, type Ring } from './mvt';
import { buildGrid, type Grid } from './roads';
import { AREAS, DEFAULT_HEIGHT, LINE, ROADS, TALL, facadeColor, roadWidth } from './style';

export type FlatMesh = { pos: Float32Array; col: Uint8Array; idx: Uint32Array };
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
  ground: FlatMesh;
  roads: FlatMesh;
  buildings: BuildingChunk[];
  /** x, z, heading for each street lamp. */
  lamps: Float32Array;
  /** x, z, size for each tree. */
  trees: Float32Array;
  /** Drivable road centre segments: x1, z1, x2, z2. */
  segs: Float32Array;
  /** Half width of each segment's road, in meters. */
  segHalf: Float32Array;
  names: string[];
  /** Named road segments: x1, z1, x2, z2, with nameOf[i] into names. */
  nameSegs: Float32Array;
  nameOf: Uint16Array;
  /** Building outline edges cars bounce off: x1, z1, x2, z2. */
  walls: Float32Array;
  /** West and north edges and size of the tile, in meters. */
  bounds: [number, number, number];
  /** Lookup grids over segs, nameSegs and walls. */
  grids: [Grid, Grid, Grid];
  stats: { buildings: number; roads: number; triangles: number };
};

export const CHUNKS = 4;
/** The window texture repeats every eight windows of four meters, and every eight floors. */
export const WINDOW_TILE_U = 32;
export const WINDOW_TILE_V = 28;
const GROUND_Y = { area: 0, park: 0.02, water: 0.04 };
const ROAD_Y = 0.08;
const LINE_Y = 0.1;
const MAX_LAMPS = 600;
const MAX_TREES = 500;
const LAMP_EVERY = 38;

/** Vertex colours are read as linear, so sRGB bytes are converted once here. */
const LINEAR = Array.from({ length: 256 }, (_, i) => {
  const c = i / 255;
  return Math.round((c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4) * 255);
});

class Mesh {
  pos: number[] = [];
  col: number[] = [];
  idx: number[] = [];

  vertex(x: number, y: number, z: number, c: readonly number[]) {
    this.pos.push(x, y, z);
    this.col.push(LINEAR[Math.round(c[0])], LINEAR[Math.round(c[1])], LINEAR[Math.round(c[2])]);
    return this.pos.length / 3 - 1;
  }

  /** Adds a flat triangle wound to face up. */
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

  chunk(): BuildingChunk {
    const tallCount = this.tall.length;
    const idx = new Uint32Array(tallCount + this.idx.length);
    idx.set(this.tall);
    idx.set(this.idx, tallCount);
    const cx = (this.minX + this.maxX) / 2;
    const cz = (this.minZ + this.maxZ) / 2;
    return {
      pos: new Float32Array(this.pos),
      col: new Uint8Array(this.col),
      idx,
      nor: new Float32Array(this.nor),
      uv: new Float32Array(this.uv),
      tallCount,
      cx,
      cz,
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

type Pt = [number, number];

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

function fillPolygon(mesh: Mesh, outer: Pt[], holes: Pt[][], y: number, color: readonly number[]) {
  if (outer.length < 3) return 0;
  const contour = outer.map(([x, z]) => new Vector2(x, z));
  const holeVs = holes.filter((h) => h.length >= 3).map((h) => h.map(([x, z]) => new Vector2(x, z)));
  const tris = ShapeUtils.triangulateShape(contour, holeVs);
  const base = mesh.pos.length / 3;
  for (const [x, z] of outer) mesh.vertex(x, y, z, color);
  for (const h of holes) if (h.length >= 3) for (const [x, z] of h) mesh.vertex(x, y, z, color);
  for (const [a, b, c] of tris) mesh.up(mesh.idx, base + a, base + b, base + c);
  return tris.length;
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

/** A ribbon along a polyline, mitred at bends and capped with a disc at each end. */
export function ribbon(mesh: Mesh, line: Pt[], half: number, y: number, color: readonly number[], caps = true) {
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
    mesh.vertex(line[i][0] + nx, y, line[i][1] + nz, color);
    mesh.vertex(line[i][0] - nx, y, line[i][1] - nz, color);
  }
  for (let i = 0; i < n - 1; i++) {
    const a = base + i * 2;
    mesh.up(mesh.idx, a, a + 2, a + 1);
    mesh.up(mesh.idx, a + 1, a + 2, a + 3);
    tris += 2;
  }
  if (caps) for (const end of [line[0], line[n - 1]]) tris += disc(mesh, end[0], end[1], half, y, color);
  return tris;
}

function disc(mesh: Mesh, x: number, z: number, r: number, y: number, color: readonly number[]) {
  const sides = 8;
  const centre = mesh.vertex(x, y, z, color);
  for (let i = 0; i < sides; i++) {
    const a = (i / sides) * Math.PI * 2;
    mesh.vertex(x + Math.cos(a) * r, y, z + Math.sin(a) * r, color);
  }
  for (let i = 0; i < sides; i++) mesh.up(mesh.idx, centre, centre + 1 + ((i + 1) % sides), centre + 1 + i);
  return sides;
}

function dashes(mesh: Mesh, line: Pt[], y: number) {
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

/** Walls and roof of one building, appended to its chunk. */
function extrude(mesh: BuildingMesh, outer: Pt[], holes: Pt[][], min: number, height: number, color: readonly number[]) {
  const tall = height >= TALL;
  const into = tall ? mesh.tall : mesh.idx;
  let tris = 0;
  const wall = (ring: Pt[]) => {
    let along = 0;
    for (let i = 0; i < ring.length; i++) {
      const [x1, z1] = ring[i];
      const [x2, z2] = ring[(i + 1) % ring.length];
      const len = Math.hypot(x2 - x1, z2 - z1);
      if (len < 0.05) continue;
      const nx = (z2 - z1) / len;
      const nz = -(x2 - x1) / len;
      const u1 = along / WINDOW_TILE_U;
      const u2 = (along + len) / WINDOW_TILE_U;
      const v1 = min / WINDOW_TILE_V;
      const v2 = height / WINDOW_TILE_V;
      const a = mesh.v(x1, min, z1, nx, 0, nz, u1, v1, color);
      mesh.v(x2, min, z2, nx, 0, nz, u2, v1, color);
      mesh.v(x2, height, z2, nx, 0, nz, u2, v2, color);
      mesh.v(x1, height, z1, nx, 0, nz, u1, v2, color);
      into.push(a, a + 2, a + 1, a, a + 3, a + 2);
      tris += 2;
      along += len;
    }
  };
  wall(outer);
  for (const h of holes) wall(h);
  const contour = outer.map(([x, z]) => new Vector2(x, z));
  const holeVs = holes.map((h) => h.map(([x, z]) => new Vector2(x, z)));
  const roof = color.map((c) => Math.min(255, c * 0.82 + 20));
  const base = mesh.pos.length / 3;
  for (const [x, z] of outer) mesh.v(x, height, z, 0, 1, 0, 0.005, 0.005, roof);
  for (const h of holes) for (const [x, z] of h) mesh.v(x, height, z, 0, 1, 0, 0.005, 0.005, roof);
  for (const [a, b, c] of ShapeUtils.triangulateShape(contour, holeVs)) {
    mesh.up(into, base + a, base + b, base + c);
    tris++;
  }
  return tris;
}

/** Makes an outer ring wind so wall normals face outward (counter clockwise seen from above, x east and z south). */
function outward(ring: Pt[], wantPositive: boolean): Pt[] {
  let area = 0;
  for (let i = 0; i < ring.length; i++) {
    const [x1, z1] = ring[i];
    const [x2, z2] = ring[(i + 1) % ring.length];
    area += x1 * z2 - x2 * z1;
  }
  return area > 0 === wantPositive ? ring : ring.slice().reverse();
}

export function buildTile(layers: Map<string, Layer>, tx: number, ty: number, frame: Frame): TileMesh {
  const ground = new Mesh();
  const roads = new Mesh();
  const lines = new Mesh();
  const chunks: BuildingMesh[] = Array.from({ length: CHUNKS * CHUNKS }, () => new BuildingMesh());
  const lamps: number[] = [];
  const trees: number[] = [];
  const segs: number[] = [];
  const segHalf: number[] = [];
  const names: string[] = [];
  const nameIndex = new Map<string, number>();
  const nameSegs: number[] = [];
  const nameOf: number[] = [];
  const walls: number[] = [];
  let triangles = 0;
  let buildingCount = 0;
  let roadCount = 0;

  const areaPass = (layerName: string, y: number, colorOf: (props: Record<string, unknown>) => readonly number[] | undefined) => {
    const layer = layers.get(layerName);
    if (!layer) return;
    for (const f of layer.features) {
      if (f.type !== 3) continue;
      const color = colorOf(f.props);
      if (!color) continue;
      for (const poly of polygons(f.parts)) {
        const outer = toWorld(frame, tx, ty, layer.extent, poly.outer);
        const holes = poly.holes.map((h) => toWorld(frame, tx, ty, layer.extent, h));
        triangles += fillPolygon(ground, outer, holes, y, color);
      }
    }
  };

  const scatterTrees = (ring: Pt[], spacing: number) => {
    let minX = Infinity;
    let maxX = -Infinity;
    let minZ = Infinity;
    let maxZ = -Infinity;
    for (const [x, z] of ring) {
      minX = Math.min(minX, x);
      maxX = Math.max(maxX, x);
      minZ = Math.min(minZ, z);
      maxZ = Math.max(maxZ, z);
    }
    for (let gx = Math.ceil(minX / spacing); gx * spacing < maxX; gx++) {
      for (let gz = Math.ceil(minZ / spacing); gz * spacing < maxZ; gz++) {
        if (trees.length / 3 >= MAX_TREES) return;
        const jx = (hash2(gx, gz) - 0.5) * spacing * 0.8;
        const jz = (hash2(gz, gx + 7) - 0.5) * spacing * 0.8;
        const x = gx * spacing + jx;
        const z = gz * spacing + jz;
        if (pointInRing(x, z, ring)) trees.push(x, z, 0.8 + hash2(gx + 3, gz) * 0.6);
      }
    }
  };

  areaPass('landuse', GROUND_Y.area, (p) => AREAS[str(p.class)]);
  areaPass('landcover', GROUND_Y.park, (p) => AREAS[str(p.subclass)] ?? AREAS[str(p.class)]);
  areaPass('park', GROUND_Y.park, () => AREAS.park);
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
  areaPass('water', GROUND_Y.water, (p) => (str(p.brunnel) === 'tunnel' ? undefined : AREAS.water));

  const transportation = layers.get('transportation');
  if (transportation) {
    const sorted = transportation.features
      .filter((f) => f.type === 2 && ROADS[str(f.props.class)] && str(f.props.brunnel) !== 'tunnel')
      .sort((a, b) => ROADS[str(a.props.class)].rank - ROADS[str(b.props.class)].rank);
    for (const f of sorted) {
      const cls = str(f.props.class);
      const style = ROADS[cls];
      const half = roadWidth(cls, num(f.props.ramp, 0) === 1) / 2;
      const y = ROAD_Y + style.rank * 0.002;
      for (const part of f.parts) {
        const line = toWorld(frame, tx, ty, transportation.extent, part);
        if (line.length < 2) continue;
        roadCount++;
        triangles += ribbon(roads, line, half, y, style.color);
        if (style.line && num(f.props.oneway, 0) === 0) triangles += dashes(lines, line, LINE_Y);
        for (let i = 0; i + 1 < line.length; i++) {
          segs.push(line[i][0], line[i][1], line[i + 1][0], line[i + 1][1]);
          segHalf.push(half);
        }
        if (style.lamps) placeLamps(line, half);
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
        if (lamps.length / 3 >= MAX_LAMPS) return;
        const off = (half + 0.8) * side;
        lamps.push(x1 + dx * t - dz * off, z1 + dz * t + dx * off, Math.atan2(dz * side, -dx * side));
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

  const building = layers.get('building');
  if (building) {
    const extent = building.extent;
    for (const f of building.features) {
      if (f.type !== 3 || f.props.hide_3d === true) continue;
      const height = Math.max(3, num(f.props.render_height, DEFAULT_HEIGHT));
      const min = Math.max(0, num(f.props.render_min_height, 0));
      if (height - min < 1) continue;
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
        // A building drawn by the tile its middle falls in, so neighbours never draw it twice.
        if (cx < 0 || cy < 0 || cx >= extent || cy >= extent) continue;
        const outer = outward(toWorld(frame, tx, ty, extent, poly.outer), true);
        if (outer.length < 3) continue;
        const holes = poly.holes.map((h) => outward(toWorld(frame, tx, ty, extent, h), false)).filter((h) => h.length >= 3);
        const chunk = chunks[Math.min(CHUNKS - 1, Math.floor((cy / extent) * CHUNKS)) * CHUNKS + Math.min(CHUNKS - 1, Math.floor((cx / extent) * CHUNKS))];
        const [wx, wz] = outer[0];
        triangles += extrude(chunk, outer, holes, min, height, facadeColor(hash2(wx, wz) * 1000, f.props.colour));
        buildingCount++;
        if (min < 2.5) {
          for (let i = 0; i < outer.length; i++) {
            const a = outer[i];
            const b = outer[(i + 1) % outer.length];
            walls.push(a[0], a[1], b[0], b[1]);
          }
        }
      }
    }
  }

  const roadsMesh = roads;
  const lineBase = roadsMesh.pos.length / 3;
  for (const v of lines.pos) roadsMesh.pos.push(v);
  for (const c of lines.col) roadsMesh.col.push(c);
  for (const i of lines.idx) roadsMesh.idx.push(i + lineBase);

  const [minX, minZ] = frame.fromTile(tx, ty, 1, 0, 0);
  const size = frame.metersPerTile;
  const segArr = new Float32Array(segs);
  const nameArr = new Float32Array(nameSegs);
  const wallArr = new Float32Array(walls);
  return {
    x: tx,
    y: ty,
    ground: ground.done(),
    roads: roadsMesh.done(),
    buildings: chunks.filter((c) => c.pos.length > 0).map((c) => c.chunk()),
    lamps: new Float32Array(lamps),
    trees: new Float32Array(trees),
    segs: segArr,
    segHalf: new Float32Array(segHalf),
    names,
    nameSegs: nameArr,
    nameOf: new Uint16Array(nameOf),
    walls: wallArr,
    bounds: [minX, minZ, size],
    grids: [buildGrid(segArr, minX, minZ, size), buildGrid(nameArr, minX, minZ, size), buildGrid(wallArr, minX, minZ, size)],
    stats: { buildings: buildingCount, roads: roadCount, triangles },
  };
}

/** Every array buffer in a tile mesh, for handing it to the main thread without a copy. */
export function transferables(t: TileMesh): ArrayBuffer[] {
  const out: ArrayBuffer[] = [];
  const add = (a: { buffer: ArrayBufferLike }) => out.push(a.buffer as ArrayBuffer);
  for (const m of [t.ground, t.roads]) [m.pos, m.col, m.idx].forEach(add);
  for (const c of t.buildings) [c.pos, c.col, c.idx, c.nor, c.uv].forEach(add);
  [t.lamps, t.trees, t.segs, t.segHalf, t.nameSegs, t.nameOf, t.walls].forEach(add);
  for (const g of t.grids) [g.start, g.items].forEach(add);
  return out;
}
