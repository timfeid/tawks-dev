/**
 * Ground heights from Terrarium elevation tiles. Each vector tile gets a grid
 * of heights taken straight from pixels of the zoom 13 elevation tiles, two
 * pixels apart, so neighbouring tiles read the very same pixels along their
 * shared edge and the ground never cracks.
 */

export const TERRAIN_ZOOM = 13;
/** Elevation pixels across one zoom 14 vector tile. */
const PIXELS_PER_TILE = 2 ** TERRAIN_ZOOM * 256 / 2 ** 14;
/** Vertices along each side of a tile's ground grid. */
export const GRID_N = 65;
const STEP = PIXELS_PER_TILE / (GRID_N - 1);

export type Heights = {
  /** Vertices per side. */
  n: number;
  /** Row major from the north west corner, meters above the spawn point. */
  h: Float32Array;
  /** West and north edges and size, in meters. */
  bounds: [number, number, number];
};

/** Meters above sea level from one Terrarium pixel; the sea floor reads as sea level. */
export function terrariumHeight(r: number, g: number, b: number): number {
  return Math.max(0, r * 256 + g + b / 256 - 32768);
}

export function decodeTerrarium(rgba: Uint8ClampedArray | Uint8Array): Float32Array {
  const out = new Float32Array(rgba.length / 4);
  for (let i = 0; i < out.length; i++) out[i] = terrariumHeight(rgba[i * 4], rgba[i * 4 + 1], rgba[i * 4 + 2]);
  return out;
}

/** The elevation tiles a vector tile's grid reads, as zoom 13 tile coordinates. */
export function elevationTilesFor(tx: number, ty: number): [number, number][] {
  const x0 = Math.floor((tx * PIXELS_PER_TILE) / 256);
  const y0 = Math.floor((ty * PIXELS_PER_TILE) / 256);
  const x1 = Math.floor(((tx + 1) * PIXELS_PER_TILE) / 256);
  const y1 = Math.floor(((ty + 1) * PIXELS_PER_TILE) / 256);
  const out: [number, number][] = [];
  for (let y = y0; y <= y1; y++) for (let x = x0; x <= x1; x++) out.push([x, y]);
  return out;
}

/** Height at a global zoom 13 pixel, read from whichever elevation tile holds it. */
export type PixelHeight = (px: number, py: number) => number;

export function heightsForTile(tx: number, ty: number, pixel: PixelHeight, base: number, bounds: [number, number, number]): Heights {
  const h = new Float32Array(GRID_N * GRID_N);
  for (let j = 0; j < GRID_N; j++) {
    for (let i = 0; i < GRID_N; i++) {
      h[j * GRID_N + i] = pixel(tx * PIXELS_PER_TILE + i * STEP, ty * PIXELS_PER_TILE + j * STEP) - base;
    }
  }
  return { n: GRID_N, h, bounds };
}

/** A flat grid for places without elevation, or with terrain turned off. */
export function flatHeights(bounds: [number, number, number]): Heights {
  return { n: 2, h: new Float32Array(4), bounds };
}

/**
 * Height at a world point, on the same two triangles per grid cell the ground
 * mesh draws (split from north west to south east), so cars sit on what is drawn.
 */
export function heightAt(t: Heights, x: number, z: number): number {
  const [minX, minZ, size] = t.bounds;
  const cells = t.n - 1;
  const fx = Math.min(cells, Math.max(0, ((x - minX) / size) * cells));
  const fz = Math.min(cells, Math.max(0, ((z - minZ) / size) * cells));
  const i = Math.min(cells - 1, Math.floor(fx));
  const j = Math.min(cells - 1, Math.floor(fz));
  const u = fx - i;
  const v = fz - j;
  const h = t.h;
  const n = t.n;
  const a = h[j * n + i];
  const b = h[j * n + i + 1];
  const c = h[(j + 1) * n + i];
  const d = h[(j + 1) * n + i + 1];
  return u >= v ? a + (b - a) * u + (d - b) * v : a + (d - c) * u + (c - a) * v;
}
