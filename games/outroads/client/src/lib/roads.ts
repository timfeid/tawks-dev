/** Road, name and wall lookups over the tiles that are loaded, on a coarse grid. */

import { heightAt, type Heights } from './terrain';

/** Each tile's segments are bucketed on a GRID by GRID grid, built in the worker. */
export const GRID = 32;

export type Grid = { start: Uint32Array; items: Uint32Array };

export type TileData = {
  segs: Float32Array;
  segHalf: Float32Array;
  names: string[];
  nameSegs: Float32Array;
  nameOf: Uint16Array;
  walls: Float32Array;
  fillerWalls: Float32Array;
  /** Deck heights at both ends of bridge segments, NaN elsewhere. */
  segDeck: Float32Array;
  heights: Heights;
  /** West and north edges of the tile, and its size, in meters. */
  bounds: [number, number, number];
  grids: [Grid, Grid, Grid, Grid];
};

type Kind = 0 | 1 | 2 | 3;

/** Buckets segments (x1, z1, x2, z2) by the grid cells their bounding boxes touch. */
export function buildGrid(segs: Float32Array, minX: number, minZ: number, size: number): Grid {
  const cell = size / GRID;
  const clamp = (v: number) => Math.max(0, Math.min(GRID - 1, Math.floor(v)));
  const count = new Uint32Array(GRID * GRID + 1);
  const span = (i: number) => {
    const ax = clamp((Math.min(segs[i], segs[i + 2]) - minX) / cell);
    const bx = clamp((Math.max(segs[i], segs[i + 2]) - minX) / cell);
    const az = clamp((Math.min(segs[i + 1], segs[i + 3]) - minZ) / cell);
    const bz = clamp((Math.max(segs[i + 1], segs[i + 3]) - minZ) / cell);
    return [ax, bx, az, bz];
  };
  for (let i = 0; i + 3 < segs.length; i += 4) {
    const [ax, bx, az, bz] = span(i);
    for (let cz = az; cz <= bz; cz++) for (let cx = ax; cx <= bx; cx++) count[cz * GRID + cx + 1]++;
  }
  for (let c = 1; c <= GRID * GRID; c++) count[c] += count[c - 1];
  const items = new Uint32Array(count[GRID * GRID]);
  const fill = count.slice(0, GRID * GRID);
  for (let i = 0; i + 3 < segs.length; i += 4) {
    const [ax, bx, az, bz] = span(i);
    for (let cz = az; cz <= bz; cz++) for (let cx = ax; cx <= bx; cx++) items[fill[cz * GRID + cx]++] = i / 4;
  }
  return { start: count, items };
}

type Loaded = { data: TileData; stamps: [Uint32Array, Uint32Array, Uint32Array, Uint32Array] };

export type RoadHit = {
  dist: number;
  /** The closest point on the road's centre line. */
  px: number;
  pz: number;
  /** Unit direction of the segment. */
  dx: number;
  dz: number;
  half: number;
};

/** Closest point on segment (x1, z1)-(x2, z2) to (x, z), as [px, pz, t]. */
export function closestOnSegment(x: number, z: number, x1: number, z1: number, x2: number, z2: number): [number, number, number] {
  const dx = x2 - x1;
  const dz = z2 - z1;
  const len2 = dx * dx + dz * dz;
  const t = len2 > 0 ? Math.max(0, Math.min(1, ((x - x1) * dx + (z - z1) * dz) / len2)) : 0;
  return [x1 + dx * t, z1 + dz * t, t];
}

export class RoadIndex {
  private tiles = new Map<string, Loaded>();
  private stamp = 0;
  /** Whether made up houses are solid. */
  fillers = false;

  get tileCount() {
    return this.tiles.size;
  }

  add(key: string, data: TileData) {
    this.tiles.set(key, {
      data,
      stamps: [
        new Uint32Array(data.segHalf.length),
        new Uint32Array(data.nameOf.length),
        new Uint32Array(data.walls.length / 4),
        new Uint32Array(data.fillerWalls.length / 4),
      ],
    });
  }

  remove(key: string) {
    this.tiles.delete(key);
  }

  clear() {
    this.tiles.clear();
  }

  /** Visits each segment of a kind whose cell is within r of a point, once per query. */
  private near(kind: Kind, x: number, z: number, r: number, visit: (tile: TileData, i: number) => void) {
    const stamp = ++this.stamp;
    for (const { data, stamps } of this.tiles.values()) {
      const [minX, minZ, size] = data.bounds;
      const margin = size * 0.05;
      if (x + r < minX - margin || x - r > minX + size + margin || z + r < minZ - margin || z - r > minZ + size + margin) continue;
      const cell = size / GRID;
      const clamp = (v: number) => Math.max(0, Math.min(GRID - 1, Math.floor(v)));
      const ax = clamp((x - r - minX) / cell);
      const bx = clamp((x + r - minX) / cell);
      const az = clamp((z - r - minZ) / cell);
      const bz = clamp((z + r - minZ) / cell);
      const grid = data.grids[kind];
      const seen = stamps[kind];
      for (let cz = az; cz <= bz; cz++) {
        for (let cx = ax; cx <= bx; cx++) {
          const c = cz * GRID + cx;
          for (let k = grid.start[c]; k < grid.start[c + 1]; k++) {
            const i = grid.items[k];
            if (seen[i] === stamp) continue;
            seen[i] = stamp;
            visit(data, i);
          }
        }
      }
    }
  }

  nearestRoad(x: number, z: number, maxDist = 400): RoadHit | null {
    let best = null as RoadHit | null;
    for (let r = Math.min(64, maxDist); ; r = Math.min(maxDist, r * 2)) {
      this.near(0, x, z, r, (tile, i) => {
        const s = tile.segs;
        const o = i * 4;
        const [px, pz] = closestOnSegment(x, z, s[o], s[o + 1], s[o + 2], s[o + 3]);
        const dist = Math.hypot(px - x, pz - z);
        if (dist <= maxDist && (!best || dist < best.dist)) {
          const len = Math.hypot(s[o + 2] - s[o], s[o + 3] - s[o + 1]) || 1;
          best = { dist, px, pz, dx: (s[o + 2] - s[o]) / len, dz: (s[o + 3] - s[o + 1]) / len, half: tile.segHalf[i] };
        }
      });
      if ((best && best.dist <= r) || r >= maxDist) return best;
    }
  }

  /** Every road segment within r, closest point first, for picking a good spawn. */
  roadsNear(x: number, z: number, r: number): RoadHit[] {
    const out: RoadHit[] = [];
    this.near(0, x, z, r, (tile, i) => {
      const s = tile.segs;
      const o = i * 4;
      const [px, pz] = closestOnSegment(x, z, s[o], s[o + 1], s[o + 2], s[o + 3]);
      const dist = Math.hypot(px - x, pz - z);
      if (dist > r) return;
      const len = Math.hypot(s[o + 2] - s[o], s[o + 3] - s[o + 1]) || 1;
      out.push({ dist, px, pz, dx: (s[o + 2] - s[o]) / len, dz: (s[o + 3] - s[o + 1]) / len, half: tile.segHalf[i] });
    });
    return out.sort((a, b) => a.dist - b.dist);
  }

  /** How far a ray goes before it meets a wall, up to max. */
  clearance(x: number, z: number, dx: number, dz: number, max: number): number {
    let best = max;
    const cx = x + (dx * max) / 2;
    const cz = z + (dz * max) / 2;
    this.walls(cx, cz, max / 2 + 1, (x1, z1, x2, z2) => {
      const ex = x2 - x1;
      const ez = z2 - z1;
      const den = dx * ez - dz * ex;
      if (Math.abs(den) < 1e-9) return;
      const t = ((x1 - x) * ez - (z1 - z) * ex) / den;
      const u = ((x1 - x) * dz - (z1 - z) * dx) / den;
      if (t >= 0 && u >= 0 && u <= 1 && t < best) best = t;
    });
    return best;
  }

  streetName(x: number, z: number, maxDist = 30): string | null {
    let best = maxDist;
    let name: string | null = null;
    this.near(1, x, z, maxDist, (tile, i) => {
      const s = tile.nameSegs;
      const o = i * 4;
      const [px, pz] = closestOnSegment(x, z, s[o], s[o + 1], s[o + 2], s[o + 3]);
      const dist = Math.hypot(px - x, pz - z);
      if (dist < best) {
        best = dist;
        name = tile.names[tile.nameOf[i]];
      }
    });
    return name;
  }

  /** Calls `visit` with each wall segment near a point. */
  walls(x: number, z: number, r: number, visit: (x1: number, z1: number, x2: number, z2: number) => void) {
    this.near(2, x, z, r, (tile, i) => {
      const w = tile.walls;
      const o = i * 4;
      visit(w[o], w[o + 1], w[o + 2], w[o + 3]);
    });
    if (this.fillers)
      this.near(3, x, z, r, (tile, i) => {
        const w = tile.fillerWalls;
        const o = i * 4;
        visit(w[o], w[o + 1], w[o + 2], w[o + 3]);
      });
  }

  private tileAt(x: number, z: number): TileData | null {
    for (const { data } of this.tiles.values()) {
      const [minX, minZ, size] = data.bounds;
      if (x >= minX && z >= minZ && x <= minX + size && z <= minZ + size) return data;
    }
    return null;
  }

  /** Ground height, or 0 where no tile has loaded. */
  terrain(x: number, z: number): number {
    const tile = this.tileAt(x, z);
    return tile ? heightAt(tile.heights, x, z) : 0;
  }

  /**
   * What a car at height `y` stands on: a bridge deck it is on or just above,
   * else the ground.
   */
  groundAt(x: number, z: number, y = Infinity): number {
    let best = this.terrain(x, z);
    this.near(0, x, z, 16, (tile, i) => {
      const d1 = tile.segDeck[i * 2];
      if (Number.isNaN(d1)) return;
      const s = tile.segs;
      const o = i * 4;
      const [px, pz, t] = closestOnSegment(x, z, s[o], s[o + 1], s[o + 2], s[o + 3]);
      if (Math.hypot(px - x, pz - z) > tile.segHalf[i] + 1) return;
      const deck = d1 + (tile.segDeck[i * 2 + 1] - d1) * t;
      if (deck > best && y >= deck - 2) best = deck;
    });
    return best;
  }

  /** Each road and named segment within a square, for the minimap. */
  forEachRoad(x: number, z: number, r: number, visit: (s: Float32Array, o: number, half: number) => void) {
    this.near(0, x, z, r, (tile, i) => visit(tile.segs, i * 4, tile.segHalf[i]));
  }

  forEachName(x: number, z: number, r: number, visit: (name: string, s: Float32Array, o: number) => void) {
    this.near(1, x, z, r, (tile, i) => visit(tile.names[tile.nameOf[i]], tile.nameSegs, i * 4));
  }
}
