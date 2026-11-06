import { describe, expect, it } from 'vitest';
import { Frame, latLonOfTile } from './geo';
import { decodeTile, polygons, ringArea } from './mvt';
import { RoadIndex } from './roads';
import { encodeTile } from './testing';
import { CHUNKS, buildTile, clipLine, groundMesh } from './tilemesh';
import { decodeTerrarium, elevationTilesFor, heightAt, heightsForTile, terrariumHeight } from './terrain';

const TX = 4824;
const TY = 6158;
const corner = latLonOfTile(TX, TY);
const frame = new Frame(corner);

/** A square in tile units, wound the way vector tiles mark an outer ring. */
const square = (x: number, y: number, s: number) => [x, y, x + s, y, x + s, y + s, x, y + s, x, y];

function upward(pos: Float32Array, idx: Uint32Array) {
  for (let i = 0; i < idx.length; i += 3) {
    const [a, b, c] = [idx[i] * 3, idx[i + 1] * 3, idx[i + 2] * 3];
    const cross = (pos[b + 2] - pos[a + 2]) * (pos[c] - pos[a]) - (pos[b] - pos[a]) * (pos[c + 2] - pos[a + 2]);
    if (cross < -1e-6) return false;
  }
  return true;
}

describe('vector tile decoding', () => {
  it('reads layers, properties and rings', () => {
    const buf = encodeTile({
      building: [{ type: 3, props: { render_height: 42.5, colour: '#aa3300' }, parts: [square(100, 100, 200)] }],
      transportation: [{ type: 2, props: { class: 'primary', oneway: 0 }, parts: [[0, 2048, 4096, 2048]] }],
    });
    const layers = decodeTile(buf);
    expect([...layers.keys()]).toEqual(['building', 'transportation']);
    const b = layers.get('building')!.features[0];
    expect(b.type).toBe(3);
    expect(b.props.render_height).toBe(42.5);
    expect(b.props.colour).toBe('#aa3300');
    expect(b.parts[0].slice(0, 4)).toEqual([100, 100, 300, 100]);
    expect(ringArea(b.parts[0])).toBeGreaterThan(0);
    const road = layers.get('transportation')!.features[0];
    expect(road.parts).toEqual([[0, 2048, 4096, 2048]]);
    expect(decodeTile(buf, ['building']).size).toBe(1);
  });

  it('puts holes with the ring before them', () => {
    const outer = square(0, 0, 1000);
    const hole = [200, 200, 200, 400, 400, 400, 400, 200, 200, 200];
    const groups = polygons([outer, hole, square(2000, 2000, 10)]);
    expect(groups.length).toBe(2);
    expect(groups[0].holes.length).toBe(1);
  });
});

describe('tile to mesh', () => {
  const layers = decodeTile(
    encodeTile({
      water: [{ type: 3, props: { class: 'lake' }, parts: [square(3000, 3000, 500)] }],
      park: [{ type: 3, props: { class: 'park' }, parts: [square(100, 3000, 800)] }],
      transportation: [
        { type: 2, props: { class: 'primary', oneway: 0 }, parts: [[0, 2048, 2048, 2048, 4096, 2100]] },
        { type: 2, props: { class: 'path' }, parts: [[0, 0, 100, 100]] },
        { type: 2, props: { class: 'minor', brunnel: 'tunnel' }, parts: [[0, 10, 100, 10]] },
      ],
      transportation_name: [{ type: 2, props: { name: 'Grand Street', class: 'primary' }, parts: [[0, 2048, 4096, 2048]] }],
      building: [
        { type: 3, props: { render_height: 120 }, parts: [square(500, 500, 100)] },
        { type: 3, props: { render_height: 6 }, parts: [square(3500, 500, 60)] },
        { type: 3, props: {}, parts: [square(4090, 900, 100)] },
        { type: 3, props: { render_height: 30, hide_3d: true }, parts: [square(1500, 1500, 60)] },
      ],
    }),
  );
  const mesh = buildTile(layers, TX, TY, frame);

  it('places the tile in meters from the origin', () => {
    const size = frame.metersPerTile;
    expect(size).toBeGreaterThan(1700);
    expect(size).toBeLessThan(2500);
    const xs = mesh.segs.filter((_, i) => i % 2 === 0);
    expect(Math.min(...xs)).toBeCloseTo(0, 3);
    expect(Math.max(...xs)).toBeCloseTo(size, 1);
  });

  it('draws only drivable roads that are above ground', () => {
    expect(mesh.stats.roads).toBe(1);
    expect(mesh.segs.length / 4).toBe(2);
    expect(mesh.segHalf[0]).toBeCloseTo(5.75);
    expect(mesh.roads.idx.length).toBeGreaterThan(0);
    expect(upward(mesh.roads.pos, mesh.roads.idx)).toBe(true);
    expect(mesh.lamps.length / 4).toBeGreaterThan(20);
  });

  it('extrudes buildings at their heights, tall ones first, and skips the ones it should', () => {
    expect(mesh.stats.buildings).toBe(2);
    const heights = mesh.buildings.flatMap((c) => Array.from(c.pos.filter((_, i) => i % 3 === 1)));
    expect(Math.max(...heights)).toBeCloseTo(120);
    expect(heights).toContain(6);
    const tallChunk = mesh.buildings.find((c) => c.tallCount > 0)!;
    expect(tallChunk.tallCount).toBe(tallChunk.idx.length);
    const shortChunk = mesh.buildings.find((c) => c !== tallChunk)!;
    expect(shortChunk.tallCount).toBe(0);
    expect(mesh.buildings.length).toBeLessThanOrEqual(CHUNKS * CHUNKS);
    expect(mesh.walls.length / 4).toBe(8);
  });

  it('points walls outward and roofs up', () => {
    const c = mesh.buildings.find((b) => b.tallCount > 0)!;
    let cx = 0;
    let cz = 0;
    const n = c.pos.length / 3;
    for (let i = 0; i < n; i++) {
      cx += c.pos[i * 3] / n;
      cz += c.pos[i * 3 + 2] / n;
    }
    for (let i = 0; i < n; i++) {
      const nx = c.nor[i * 3];
      const ny = c.nor[i * 3 + 1];
      const nz = c.nor[i * 3 + 2];
      if (ny > 0.5) continue;
      expect((c.pos[i * 3] - cx) * nx + (c.pos[i * 3 + 2] - cz) * nz).toBeGreaterThan(0);
    }
    for (let i = 0; i < c.idx.length; i += 3) {
      const [a, b, d] = [c.idx[i] * 3, c.idx[i + 1] * 3, c.idx[i + 2] * 3];
      const ux = c.pos[b] - c.pos[a], uy = c.pos[b + 1] - c.pos[a + 1], uz = c.pos[b + 2] - c.pos[a + 2];
      const vx = c.pos[d] - c.pos[a], vy = c.pos[d + 1] - c.pos[a + 1], vz = c.pos[d + 2] - c.pos[a + 2];
      const face = [uy * vz - uz * vy, uz * vx - ux * vz, ux * vy - uy * vx];
      const normal = [c.nor[a], c.nor[a + 1], c.nor[a + 2]];
      expect(face[0] * normal[0] + face[1] * normal[1] + face[2] * normal[2]).toBeGreaterThan(0);
    }
  });

  it('lays a flat ground grid facing up, with trees in the park', () => {
    expect(mesh.ground.pos.length / 3).toBe(4);
    expect(upward(mesh.ground.pos, mesh.ground.idx)).toBe(true);
    expect(mesh.trees.length / 4).toBeGreaterThan(5);
  });

  it('finds the road and its name from a point beside it', () => {
    const index = new RoadIndex();
    index.add('t', mesh);
    const [x, z] = frame.fromTile(TX, TY, 4096, 1000, 2048);
    const hit = index.nearestRoad(x + 3, z + 20)!;
    expect(hit.dist).toBeCloseTo(20, 0);
    expect(Math.abs(hit.dx)).toBeCloseTo(1, 2);
    expect(index.streetName(x, z + 10)).toBe('Grand Street');
    expect(index.streetName(x, z + 300)).toBeNull();
    let walls = 0;
    const [bx, bz] = frame.fromTile(TX, TY, 4096, 550, 500);
    index.walls(bx, bz, 5, () => walls++);
    expect(walls).toBeGreaterThan(0);
    let roads = 0;
    index.forEachRoad(x, z, 3000, () => roads++);
    expect(roads).toBe(mesh.segs.length / 4);
    index.remove('t');
    expect(index.nearestRoad(x, z)).toBeNull();
  });
});

describe('houses', () => {
  const house = (x: number, y: number, w: number, d: number) => [x, y, x + w, y, x + w, y + d, x, y + d, x, y];
  const layers = decodeTile(
    encodeTile({
      landuse: [{ type: 3, props: { class: 'industrial' }, parts: [square(0, 2600, 800)] }],
      transportation: [
        { type: 2, props: { class: 'minor' }, parts: [[0, 1000, 4096, 1000]] },
        { type: 2, props: { class: 'minor' }, parts: [[0, 3000, 4096, 3000]] },
      ],
      building: [
        // Unmapped heights come through as 5 m; small ones are houses.
        { type: 3, props: { render_height: 5, render_min_height: 0 }, parts: [house(200, 1020, 20, 20)] },
        { type: 3, props: { render_height: 5, render_min_height: 0 }, parts: [house(400, 1020, 80, 60)] },
        { type: 3, props: { render_height: 12, render_min_height: 0 }, parts: [house(600, 1020, 20, 20)] },
      ],
    }),
  );

  it('gives small unmapped buildings house heights and gable roofs', () => {
    const mesh = buildTile(layers, TX, TY, frame);
    expect(mesh.stats.houses).toBe(1);
    const ys = mesh.buildings.flatMap((c) => Array.from(c.pos.filter((_, i) => i % 3 === 1)));
    const top = Math.max(...ys);
    expect(top).toBeGreaterThan(7);
    expect(top).toBeLessThan(13);
    expect(ys).toContain(12);
    const slanted = mesh.buildings.some((c) => {
      for (let i = 1; i < c.nor.length; i += 3) if (c.nor[i] > 0.2 && c.nor[i] < 0.95) return true;
      return false;
    });
    expect(slanted).toBe(true);
  });

  it('keeps flat roofs on rowhouses standing wall to wall', () => {
    const row = decodeTile(
      encodeTile({
        building: [0, 1, 2, 3].map((i) => ({ type: 3 as const, props: { render_height: 5, render_min_height: 0 }, parts: [house(1000 + i * 13, 1000, 13, 36)] })),
      }),
    );
    const mesh = buildTile(row, TX, TY, frame);
    expect(mesh.stats.houses).toBe(0);
    expect(mesh.stats.buildings).toBe(4);
    const tops = mesh.buildings.flatMap((c) => Array.from(c.pos.filter((_, i) => i % 3 === 1)));
    expect(Math.max(...tops)).toBeGreaterThan(9);
  });

  it('fills unmapped streets with plain houses, clear of real ones, the road and other land use', () => {
    const none = buildTile(layers, TX, TY, frame);
    expect(none.fillers.length).toBe(0);
    const filled = buildTile(layers, TX, TY, frame, null, { fillers: true });
    expect(filled.stats.fillers).toBeGreaterThan(100);
    expect(filled.fillerWalls.length / 16).toBe(filled.stats.fillers);
    const index = new RoadIndex();
    index.add('t', filled);
    const w = filled.fillerWalls;
    for (let i = 0; i < w.length; i += 4) {
      const hit = index.nearestRoad(w[i], w[i + 1], 60)!;
      expect(hit.dist).toBeGreaterThan(hit.half + 1);
    }
    const [rx, rz] = frame.fromTile(TX, TY, 4096, 210, 1030);
    for (let i = 0; i < w.length; i += 4) expect(Math.hypot(w[i] - rx, w[i + 1] - rz)).toBeGreaterThan(8);
    const [ix0, iz0] = frame.fromTile(TX, TY, 4096, 0, 2600);
    const [ix1, iz1] = frame.fromTile(TX, TY, 4096, 800, 3400);
    for (let i = 0; i < w.length; i += 4) expect(w[i] > ix0 + 1 && w[i] < ix1 - 1 && w[i + 1] > iz0 + 1 && w[i + 1] < iz1 - 1).toBe(false);
  });
});

describe('clipping lines to a tile', () => {
  it('keeps only the inside, capping only real ends', () => {
    const pieces = clipLine(
      [
        [-10, 5],
        [5, 5],
        [5, 20],
        [5, 5],
        [8, 8],
      ],
      0,
      0,
      10,
      10,
    );
    expect(pieces.length).toBe(2);
    expect(pieces[0].line[0]).toEqual([0, 5]);
    expect(pieces[0].startCap).toBe(false);
    expect(pieces[0].line[pieces[0].line.length - 1]).toEqual([5, 10]);
    expect(pieces[1].line[0]).toEqual([5, 10]);
    expect(pieces[1].endCap).toBe(true);
    expect(clipLine([[20, 20], [30, 30]], 0, 0, 10, 10)).toEqual([]);
  });
});

describe('terrain', () => {
  const bounds = (tx: number, ty: number): [number, number, number] => {
    const [x, z] = frame.fromTile(tx, ty, 1, 0, 0);
    return [x, z, frame.metersPerTile];
  };
  // A hill rising to the east, read as global elevation pixels.
  const pixel = (px: number, py: number) => (px % 1000) * 0.4 + Math.sin(py / 30) * 3;

  it('reads Terrarium pixels and treats the sea floor as sea level', () => {
    expect(terrariumHeight(128, 0, 0)).toBe(0);
    expect(terrariumHeight(128, 100, 128)).toBeCloseTo(100.5);
    expect(terrariumHeight(100, 0, 0)).toBe(0);
    expect(Array.from(decodeTerrarium(new Uint8Array([128, 10, 0, 255, 129, 0, 0, 255])))).toEqual([10, 256]);
  });

  it('gives neighbouring tiles identical heights along their shared edge', () => {
    const a = heightsForTile(TX, TY, pixel, 0, bounds(TX, TY));
    const b = heightsForTile(TX + 1, TY, pixel, 0, bounds(TX + 1, TY));
    const c = heightsForTile(TX, TY + 1, pixel, 0, bounds(TX, TY + 1));
    const n = a.n;
    for (let j = 0; j < n; j++) expect(a.h[j * n + n - 1]).toBe(b.h[j * n]);
    for (let i = 0; i < n; i++) expect(a.h[(n - 1) * n + i]).toBe(c.h[i]);
    expect(elevationTilesFor(TX, TY).length).toBeGreaterThanOrEqual(1);
  });

  it('samples heights on the same triangles the ground mesh draws', () => {
    const t = heightsForTile(TX, TY, pixel, 0, bounds(TX, TY));
    const mesh = groundMesh(t);
    for (let k = 0; k < 200; k++) {
      const v = (k * 37) % (mesh.pos.length / 3);
      expect(heightAt(t, mesh.pos[v * 3], mesh.pos[v * 3 + 2])).toBeCloseTo(mesh.pos[v * 3 + 1], 4);
    }
    const [x0, z0, size] = t.bounds;
    const cell = size / (t.n - 1);
    const mid = heightAt(t, x0 + cell * 10.25, z0 + cell * 20.75);
    const lo = Math.min(t.h[20 * t.n + 10], t.h[21 * t.n + 11], t.h[21 * t.n + 10], t.h[20 * t.n + 11]);
    const hi = Math.max(t.h[20 * t.n + 10], t.h[21 * t.n + 11], t.h[21 * t.n + 10], t.h[20 * t.n + 11]);
    expect(mid).toBeGreaterThanOrEqual(lo - 1e-4);
    expect(mid).toBeLessThanOrEqual(hi + 1e-4);
  });

  it('stands buildings on the lowest ground under them and drapes roads over the hill', () => {
    const layers = decodeTile(
      encodeTile({
        transportation: [{ type: 2, props: { class: 'primary' }, parts: [[0, 2048, 4096, 2048]] }],
        building: [{ type: 3, props: { render_height: 20 }, parts: [square(1000, 1000, 300)] }],
      }),
    );
    const t = heightsForTile(TX, TY, pixel, 0, bounds(TX, TY));
    const mesh = buildTile(layers, TX, TY, frame, t);
    const corners = [
      [1000, 1000],
      [1300, 1000],
      [1300, 1300],
      [1000, 1300],
    ].map(([px, py]) => {
      const [x, z] = frame.fromTile(TX, TY, 4096, px, py);
      return heightAt(t, x, z);
    });
    const floor = Math.min(...corners);
    const ys = Array.from(mesh.buildings[0].pos.filter((_, i) => i % 3 === 1));
    expect(Math.min(...ys)).toBeCloseTo(floor - 0.5, 3);
    expect(Math.max(...ys)).toBeCloseTo(floor + 20, 3);
    const r = mesh.roads.pos;
    let worst = 0;
    for (let i = 0; i < r.length; i += 3) worst = Math.max(worst, Math.abs(r[i + 1] - heightAt(t, r[i], r[i + 2]) - 0.27));
    expect(worst).toBeLessThan(0.2);
  });

  it('runs a bridge straight between its ends over a dip', () => {
    const layers = decodeTile(
      encodeTile({ transportation: [{ type: 2, props: { class: 'primary', brunnel: 'bridge' }, parts: [[1000, 2048, 2000, 2048]] }] }),
    );
    // A valley: high at both ends of the bridge, low in between.
    const valley = (px: number) => 40 - 30 * Math.exp(-(((px % 128) - 96) ** 2) / 50);
    const t = heightsForTile(TX, TY, (px) => valley(px), 0, bounds(TX, TY));
    const mesh = buildTile(layers, TX, TY, frame, t);
    const index = new RoadIndex();
    index.add('t', mesh);
    const [x, z] = frame.fromTile(TX, TY, 4096, 1500, 2048);
    expect(index.terrain(x, z)).toBeLessThan(index.groundAt(x, z, 100));
    expect(index.groundAt(x, z, 0)).toBe(index.terrain(x, z));
  });
});
