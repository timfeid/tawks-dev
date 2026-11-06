/** How each OpenMapTiles class looks and drives. Colours are linear-ish sRGB bytes. */

export type RoadStyle = { width: number; color: [number, number, number]; rank: number; lamps: boolean; line: boolean };

const asphalt: [number, number, number] = [70, 74, 82];

export const ROADS: Record<string, RoadStyle> = {
  motorway: { width: 15, color: [80, 84, 94], rank: 8, lamps: true, line: true },
  trunk: { width: 13, color: [76, 80, 90], rank: 7, lamps: true, line: true },
  primary: { width: 11.5, color: [74, 78, 88], rank: 6, lamps: true, line: true },
  secondary: { width: 10, color: asphalt, rank: 5, lamps: true, line: true },
  tertiary: { width: 9, color: asphalt, rank: 4, lamps: true, line: true },
  minor: { width: 7, color: [78, 81, 88], rank: 3, lamps: true, line: false },
  busway: { width: 7, color: [92, 52, 52], rank: 3, lamps: false, line: false },
  raceway: { width: 10, color: [80, 82, 90], rank: 3, lamps: false, line: true },
  service: { width: 4.5, color: [92, 94, 98], rank: 2, lamps: false, line: false },
  track: { width: 3.5, color: [118, 100, 76], rank: 1, lamps: false, line: false },
};

/** Ramps and links are narrower than the road they leave. */
export function roadWidth(cls: string, ramp: boolean): number {
  const style = ROADS[cls];
  if (!style) return 0;
  return ramp ? Math.min(style.width, 7.5) : style.width;
}

export const AREAS: Record<string, [number, number, number]> = {
  water: [52, 108, 150],
  park: [96, 150, 88],
  grass: [112, 158, 92],
  wood: [70, 118, 70],
  forest: [70, 118, 70],
  farmland: [168, 172, 116],
  wetland: [102, 140, 120],
  sand: [214, 198, 150],
  ice: [228, 236, 240],
  rock: [150, 146, 140],
  pitch: [86, 156, 98],
  cemetery: [124, 150, 114],
  stadium: [140, 140, 136],
  residential: [204, 196, 180],
  commercial: [206, 194, 188],
  industrial: [190, 184, 178],
  school: [186, 176, 158],
  hospital: [190, 172, 172],
  railway: [150, 144, 140],
  parking: [124, 126, 130],
};

export const GROUND: [number, number, number] = [196, 190, 172];
export const LINE: [number, number, number] = [232, 228, 210];

const facades: [number, number, number][] = [
  [196, 188, 176],
  [176, 170, 164],
  [210, 202, 186],
  [158, 152, 150],
  [186, 160, 138],
  [150, 160, 170],
  [204, 196, 202],
  [168, 142, 122],
];

/** A stable facade colour for a building, unless OSM says otherwise. */
export function facadeColor(seed: number, colour?: string | number | boolean): [number, number, number] {
  if (typeof colour === 'string' && /^#[0-9a-f]{6}$/i.test(colour)) {
    const n = parseInt(colour.slice(1), 16);
    const mix = (c: number, base: number) => Math.round(c * 0.7 + base * 0.3);
    return [mix((n >> 16) & 255, 180), mix((n >> 8) & 255, 176), mix(n & 255, 170)];
  }
  return facades[Math.abs(Math.floor(seed)) % facades.length];
}

export const DEFAULT_HEIGHT = 9;
/** OpenMapTiles gives every building without a mapped height this one. */
export const UNKNOWN_HEIGHT = 5;
/** Footprints smaller than this with no mapped height are drawn as houses. */
export const HOUSE_AREA = 360;

const sidings: [number, number, number][] = [
  [232, 228, 216],
  [214, 206, 190],
  [196, 206, 214],
  [222, 212, 176],
  [190, 176, 160],
  [206, 214, 196],
  [236, 222, 206],
  [178, 168, 160],
];
const shingles: [number, number, number][] = [
  [86, 80, 78],
  [104, 84, 72],
  [70, 74, 82],
  [120, 96, 80],
  [92, 92, 88],
];

export function sidingColor(seed: number): [number, number, number] {
  return sidings[Math.abs(Math.floor(seed)) % sidings.length];
}

export function shingleColor(seed: number): [number, number, number] {
  return shingles[Math.abs(Math.floor(seed)) % shingles.length];
}

/** Filler houses stand where OSM has none mapped, so they keep to one plain colour. */
export const FILLER_WALL: [number, number, number] = [200, 196, 188];
export const FILLER_ROOF: [number, number, number] = [112, 108, 104];
/** Buildings at least this tall stay drawn in far chunks. */
export const TALL = 18;
