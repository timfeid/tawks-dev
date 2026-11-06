import { describe, expect, it } from 'vitest';
import { Frame, latLonOfTile } from './geo';
import { decodeTile, polygons, ringArea } from './mvt';
import { RoadIndex } from './roads';
import { encodeTile } from './testing';
import { CHUNKS, buildTile } from './tilemesh';

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
    expect(mesh.lamps.length / 3).toBeGreaterThan(20);
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

  it('fills water and parks facing up, with trees in the park', () => {
    expect(mesh.ground.idx.length / 3).toBe(4);
    expect(upward(mesh.ground.pos, mesh.ground.idx)).toBe(true);
    expect(mesh.trees.length / 3).toBeGreaterThan(5);
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
