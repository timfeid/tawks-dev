/** Web Mercator tiles and a local flat frame in meters around the spawn point. */

export const TILE_ZOOM = 14;
const EARTH_R = 6378137;

export type LatLon = { lat: number; lon: number };

/** Fractional tile coordinates of a point at a zoom. */
export function tileOf({ lat, lon }: LatLon, z = TILE_ZOOM) {
  const n = 2 ** z;
  const x = ((lon + 180) / 360) * n;
  const rad = (lat * Math.PI) / 180;
  const y = ((1 - Math.log(Math.tan(rad) + 1 / Math.cos(rad)) / Math.PI) / 2) * n;
  return { x, y };
}

export function latLonOfTile(x: number, y: number, z = TILE_ZOOM): LatLon {
  const n = 2 ** z;
  const lon = (x / n) * 360 - 180;
  const lat = (Math.atan(Math.sinh(Math.PI * (1 - (2 * y) / n))) * 180) / Math.PI;
  return { lat, lon };
}

/**
 * The world frame: x east and z south, in meters, centred on the origin. Over
 * the few kilometers anyone drives, Mercator scaled by the origin's latitude
 * is true to well under a percent.
 */
export class Frame {
  readonly metersPerTile: number;
  private readonly ox: number;
  private readonly oy: number;

  constructor(readonly origin: LatLon) {
    const t = tileOf(origin);
    this.ox = t.x;
    this.oy = t.y;
    const scale = Math.cos((origin.lat * Math.PI) / 180);
    this.metersPerTile = ((2 * Math.PI * EARTH_R) / 2 ** TILE_ZOOM) * scale;
  }

  /** World meters of a point inside tile (tx, ty), given in tile units of `extent`. */
  fromTile(tx: number, ty: number, extent: number, px: number, py: number): [number, number] {
    return [(tx + px / extent - this.ox) * this.metersPerTile, (ty + py / extent - this.oy) * this.metersPerTile];
  }

  tileAt(x: number, z: number) {
    return { x: this.ox + x / this.metersPerTile, y: this.oy + z / this.metersPerTile };
  }

  toLatLon(x: number, z: number): LatLon {
    const t = this.tileAt(x, z);
    return latLonOfTile(t.x, t.y);
  }

  toWorld(p: LatLon): [number, number] {
    const t = tileOf(p);
    return [(t.x - this.ox) * this.metersPerTile, (t.y - this.oy) * this.metersPerTile];
  }
}

export const tileKey = (x: number, y: number) => `${TILE_ZOOM}/${x}/${y}`;
