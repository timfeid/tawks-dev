import * as THREE from 'three';
import type { MapKey } from '../protocol';
import type { Design, Feature } from './design';

/**
 * The land the board sits on: hills, ridges or a cone, and a beach that
 * slides under the sea the stage draws. Everything on the board asks this
 * for its height, so the path, the props and the tokens share a surface.
 */

/** How far from a space the ground is levelled, so nobody stands on a slope. */
const FLATTEN = 2.2;
/** Where the land starts dropping away, as a share of the island's reach. */
const SHORE = 0.86;
/** On a hand-built board, how far in from its coastline the land is up to full height. */
const BEACH = 9;
/** The coastline's distance field: one cell a world unit. */
const CELL = 1;

export type Extent = { cx: number; cz: number; halfW: number; halfD: number };

type Bump = { x: number; z: number; radius: number; height: number };

type Random = () => number;

/** How far inside the coastline each point of a grid is: positive on land. */
type CoastField = { field: Float32Array; minX: number; minZ: number; cols: number; rows: number };

export class Terrain {
  private bumps: Bump[] = [];
  private flats: { x: number; z: number; y: number }[] = [];
  /** The volcano's bowl, for whatever is drawn in it. */
  crater: { x: number; z: number; radius: number } | null = null;
  /** A hand-built board's land and coast, in place of the hills raised at random. */
  private features: Feature[] = [];
  /** Small islands off the coast, and how craggy the high ground is. */
  private islets: [number, number, number][] = [];
  private rough = 0;
  private coast: CoastField | null = null;

  constructor(
    private extent: Extent,
    spaces: { x: number; z: number }[],
    private waterY: number,
    seed: number,
    private map: MapKey = 'island',
    design?: Design,
  ) {
    const { cx, cz, halfW, halfD } = extent;
    let value = seed >>> 0;
    const random = () => {
      value = (value * 1664525 + 1013904223) % 4294967296;
      return value / 4294967296;
    };

    if (design) {
      this.features = design.land;
      this.islets = design.islets ?? [];
      this.rough = design.rough ?? 1.4;
      this.coast = this.measureCoast(design.coast);
      const crater = design.land.find((feature) => feature.kind === 'crater');
      if (crater && crater.kind === 'crater') this.crater = { x: crater.x, z: crater.z, radius: crater.r };
      // Spaces stand on the dry land: a river or a pool is cut round them, not under them.
      this.flats = spaces.map((space) => ({ x: space.x, z: space.z, y: this.raw(space.x, space.z, false) }));
      return;
    }
    if (map === 'volcano') this.raiseVolcano(spaces, random);
    else if (map === 'shipwreck') this.raiseCove(spaces, random);
    else if (map === 'frozen') this.raisePeak(spaces, random);
    else this.raiseIsland(spaces, random);

    // Rocks breaking the shoreline, for a silhouette that is not a dome.
    for (let i = 0; i < 5; i++) {
      const angle = random() * Math.PI * 2;
      this.bumps.push({
        x: cx + Math.cos(angle) * halfW * 0.93,
        z: cz + Math.sin(angle) * halfD * 0.93,
        radius: 2.4 + random() * 2,
        height: 1.4 + random() * 1.6,
      });
    }

    this.flats = spaces.map((space) => ({ x: space.x, z: space.z, y: this.raw(space.x, space.z) }));
  }

  /** Open ground away from the board, roomiest first. */
  private clearings(spaces: { x: number; z: number }[], least: number) {
    const { cx, cz, halfW, halfD } = this.extent;
    const found: { x: number; z: number; room: number }[] = [];
    for (let ix = 0; ix <= 44; ix++) {
      for (let iz = 0; iz <= 34; iz++) {
        const x = cx + (ix / 44 - 0.5) * 2 * halfW;
        const z = cz + (iz / 34 - 0.5) * 2 * halfD;
        if (this.reach(x, z) > 0.94) continue;
        let room = Infinity;
        for (const space of spaces) {
          room = Math.min(room, Math.hypot(x - space.x, z - space.z));
        }
        if (room > least) found.push({ x, z, room });
      }
    }
    return found.sort((a, b) => b.room - a.room);
  }

  /** A peak in the widest clearing, lesser hills, and two hills the board climbs. */
  private raiseIsland(spaces: { x: number; z: number }[], random: Random) {
    const { halfW } = this.extent;
    // Hills go wherever the board leaves room, biggest clearing first, so the
    // land rises between the paths instead of swallowing them.
    for (const clearing of this.clearings(spaces, 4.5)) {
      if (this.bumps.length >= 7) break;
      const radius = Math.min(clearing.room * 1.5, halfW * 0.42);
      if (this.bumps.some((bump) => Math.hypot(bump.x - clearing.x, bump.z - clearing.z) < bump.radius * 0.85)) continue;
      const first = this.bumps.length === 0;
      this.bumps.push({
        x: clearing.x,
        z: clearing.z,
        radius: first ? radius * 0.85 : radius,
        // The first and widest clearing carries the island's peak.
        height: first ? 9.5 : 1.8 + random() * 3,
      });
    }
    // Two hills the board itself climbs, so a turn can be uphill. The path is
    // levelled into them as it goes, which leaves a road cut around the side.
    for (let i = 0; i < 2; i++) {
      const space = spaces[Math.floor(random() * spaces.length)];
      if (!space) break;
      this.bumps.push({
        x: space.x + (random() - 0.5) * 4,
        z: space.z + (random() - 0.5) * 4,
        radius: 9 + random() * 4,
        height: 3.4 + random() * 2.2,
      });
    }
  }

  /**
   * One cone over the whole board with a bowl sunk in its top, so the lap
   * circles the foot and the crossing climbs the rim and drops into the
   * crater. A few spatter cones sit in the gaps outside it.
   */
  private raiseVolcano(spaces: { x: number; z: number }[], random: Random) {
    const { cx, cz, halfW, halfD } = this.extent;
    const radius = Math.min(halfW, halfD) * 1.08;
    this.bumps.push({ x: cx, z: cz, radius, height: 12 });
    this.crater = { x: cx, z: cz, radius: radius * 0.42 };
    this.bumps.push({ x: cx, z: cz, radius: this.crater.radius, height: -7.5 });
    for (const clearing of this.clearings(spaces, 3.5)) {
      if (this.bumps.length >= 5) break;
      if (this.reach(clearing.x, clearing.z) < 0.55) continue;
      if (this.bumps.some((bump) => bump.radius < 8 && Math.hypot(bump.x - clearing.x, bump.z - clearing.z) < 6)) continue;
      this.bumps.push({ x: clearing.x, z: clearing.z, radius: Math.min(clearing.room * 1.4, 6), height: 1.4 + random() * 1.6 });
    }
  }

  /**
   * A low sandbar: soft dunes between the paths and two rocky headlands at
   * the back corners, behind the beached hulls.
   */
  private raiseCove(spaces: { x: number; z: number }[], random: Random) {
    const { cx, cz, halfW, halfD } = this.extent;
    for (const side of [-1, 1]) {
      this.bumps.push({ x: cx + side * halfW * 0.78, z: cz - halfD * 0.9, radius: 7 + random() * 2, height: 3.6 + random() * 1.2 });
    }
    for (const clearing of this.clearings(spaces, 3.5)) {
      if (this.bumps.length >= 9) break;
      if (this.bumps.some((bump) => Math.hypot(bump.x - clearing.x, bump.z - clearing.z) < 5)) continue;
      this.bumps.push({ x: clearing.x, z: clearing.z, radius: Math.min(clearing.room * 1.4, 7), height: 0.7 + random() * 1.3 });
    }
  }

  /**
   * One tall mountain filling the board, with two shoulders off it, so the
   * lap circles the foot and the summit path goes over the top.
   */
  private raisePeak(spaces: { x: number; z: number }[], random: Random) {
    const { cx, cz, halfW, halfD } = this.extent;
    this.bumps.push({ x: cx, z: cz, radius: Math.min(halfW, halfD) * 1.12, height: 15 });
    this.bumps.push({ x: cx + halfW * 0.4, z: cz + halfD * 0.3, radius: 9, height: 3.5 + random() });
    this.bumps.push({ x: cx - halfW * 0.35, z: cz - halfD * 0.4, radius: 8, height: 3 + random() });
    for (const clearing of this.clearings(spaces, 4)) {
      if (this.bumps.length >= 6) break;
      if (this.reach(clearing.x, clearing.z) < 0.6) continue;
      this.bumps.push({ x: clearing.x, z: clearing.z, radius: Math.min(clearing.room * 1.4, 6), height: 1.2 + random() * 1.4 });
    }
  }

  /**
   * How far inside the coastline every point is, worked out once: positive on
   * land, negative out at sea. The outline is drawn smooth between its points.
   */
  private measureCoast(points: [number, number][]) {
    const outline: [number, number][] = [];
    const n = points.length;
    for (let i = 0; i < n; i++) {
      const [p0, p1, p2, p3] = [points[(i - 1 + n) % n], points[i], points[(i + 1) % n], points[(i + 2) % n]];
      for (let s = 0; s < 6; s++) {
        const t = s / 6;
        const t2 = t * t;
        const t3 = t2 * t;
        const at = (k: 0 | 1) =>
          0.5 * (2 * p1[k] + (-p0[k] + p2[k]) * t + (2 * p0[k] - 5 * p1[k] + 4 * p2[k] - p3[k]) * t2 + (-p0[k] + 3 * p1[k] - 3 * p2[k] + p3[k]) * t3);
        outline.push([at(0), at(1)]);
      }
    }
    const xs = outline.map((p) => p[0]);
    const zs = outline.map((p) => p[1]);
    const minX = Math.min(...xs) - 20;
    const minZ = Math.min(...zs) - 20;
    const cols = Math.ceil((Math.max(...xs) + 20 - minX) / CELL) + 1;
    const rows = Math.ceil((Math.max(...zs) + 20 - minZ) / CELL) + 1;
    const field = new Float32Array(cols * rows);
    for (let row = 0; row < rows; row++) {
      for (let col = 0; col < cols; col++) {
        const x = minX + col * CELL;
        const z = minZ + row * CELL;
        let inside = false;
        let nearest = Infinity;
        for (let i = 0, j = outline.length - 1; i < outline.length; j = i++) {
          const [ax, az] = outline[i];
          const [bx, bz] = outline[j];
          if (az > z !== bz > z && x < ((bx - ax) * (z - az)) / (bz - az) + ax) inside = !inside;
          const dx = bx - ax;
          const dz = bz - az;
          const t = Math.max(0, Math.min(1, ((x - ax) * dx + (z - az) * dz) / (dx * dx + dz * dz || 1)));
          nearest = Math.min(nearest, Math.hypot(x - (ax + dx * t), z - (az + dz * t)));
        }
        field[row * cols + col] = inside ? nearest : -nearest;
      }
    }
    return { field, minX, minZ, cols, rows };
  }

  /** How far inside the coastline a point is, in world units; negative at sea.
   * The line wanders a little either way of the drawn one, so it is ragged,
   * and each islet is land of its own. */
  inland(x: number, z: number) {
    const coast = this.coast;
    if (!coast) return (1 - this.reach(x, z)) * Math.min(this.extent.halfW, this.extent.halfD);
    let inland = this.coastField(coast, x, z) + noise(x * 0.07, z * 0.07) * 3.2 + noise(x * 0.21 + 17, z * 0.21) * 1.1;
    for (const [ix, iz, r] of this.islets) inland = Math.max(inland, r - Math.hypot(x - ix, z - iz) + noise(x * 0.3, z * 0.3) * 1.2);
    return inland;
  }

  private coastField(coast: CoastField, x: number, z: number) {
    const fx = Math.max(0, Math.min(coast.cols - 1.001, (x - coast.minX) / CELL));
    const fz = Math.max(0, Math.min(coast.rows - 1.001, (z - coast.minZ) / CELL));
    const col = Math.floor(fx);
    const row = Math.floor(fz);
    const tx = fx - col;
    const tz = fz - row;
    const at = (c: number, r: number) => coast.field[r * coast.cols + c];
    const top = at(col, row) * (1 - tx) + at(col + 1, row) * tx;
    const bottom = at(col, row + 1) * (1 - tx) + at(col + 1, row + 1) * tx;
    return top * (1 - tz) + bottom * tz;
  }

  /** A hand-built board's land: its own hills, ridges, plateaus, pools and rivers. */
  private shaped(x: number, z: number, wet: boolean) {
    let height = 0.4;
    for (const feature of this.features) {
      if (!wet && (feature.kind === 'pool' || feature.kind === 'river')) continue;
      let distance: number;
      if (feature.kind === 'ridge') {
        distance = toSegment(x, z, feature.from, feature.to);
      } else if (feature.kind === 'river') {
        distance = Infinity;
        for (let i = 1; i < feature.points.length; i++) {
          distance = Math.min(distance, toSegment(x, z, feature.points[i - 1], feature.points[i]));
        }
      } else {
        distance = Math.hypot(x - feature.x, z - feature.z);
      }
      if (distance >= feature.r) continue;
      const falloff = 1 - distance / feature.r;
      let shape: number;
      if (feature.kind === 'plateau') {
        // A flat top and steep sides, for cliffs.
        shape = Math.min(1, falloff * 3.2);
        shape = shape * shape * (3 - 2 * shape);
      } else if (feature.kind === 'river' || feature.kind === 'pool' || feature.kind === 'crater') {
        // A bowl with a flat bed.
        shape = Math.min(1, falloff * 2.4);
        shape = shape * shape * (3 - 2 * shape);
      } else {
        shape = Math.pow(falloff * falloff * (3 - 2 * falloff), 0.72);
      }
      height += feature.h * shape;
    }
    // Crags on the high ground, broken rock rather than smooth domes; the
    // lowlands stay gentle.
    if (this.rough > 0 && height > 1.5) {
      const crag = Math.abs(noise(x * 0.16, z * 0.16)) * 1.4 + noise(x * 0.4 + 9, z * 0.4) * 0.5;
      height += crag * this.rough * Math.min(1, (height - 1.5) / 5);
    }
    // Past the beach the ground falls away under the sea.
    const inland = this.inland(x, z);
    if (inland < BEACH) {
      const out = Math.min(1, (BEACH - inland) / (BEACH + 6));
      const lifted = Math.max(height, 0.4);
      height = lifted * (1 - out) + (this.waterY - 1.6) * out * out * 1.2;
    }
    return height;
  }

  /** How far out a point is: 0 in the middle, 1 at the island's edge. */
  private reach(x: number, z: number) {
    const dx = (x - this.extent.cx) / this.extent.halfW;
    const dz = (z - this.extent.cz) / this.extent.halfD;
    return Math.sqrt(dx * dx + dz * dz);
  }

  /** The land as the hills shape it, before the board is levelled into it.
   * Without `wet`, a hand-built board's pools and rivers are left out. */
  raw(x: number, z: number, wet = true) {
    if (this.coast) return this.shaped(x, z, wet);
    let height = 0;
    for (const bump of this.bumps) {
      const dx = x - bump.x;
      const dz = z - bump.z;
      const falloff = 1 - Math.min(1, Math.sqrt(dx * dx + dz * dz) / bump.radius);
      // Smooth at both ends, but pushed so the sides are steeper than the top.
      const shaped = Math.pow(falloff * falloff * (3 - 2 * falloff), 0.72);
      height += bump.height * shaped;
    }
    const reach = this.reach(x, z);
    if (reach > SHORE) {
      // Past the beach the ground falls away under the sea.
      const out = Math.min(1, (reach - SHORE) / 0.34);
      height = height * (1 - out) + (this.waterY - 1.2) * out * out;
    }
    return height;
  }

  /**
   * The ground at a point. Near a space the land is pulled flat to that
   * space's own height, so a tile never tilts and the path stays walkable.
   */
  height(x: number, z: number) {
    const base = this.raw(x, z);
    let weights = 0;
    let heights = 0;
    let strongest = 0;
    for (const space of this.flats) {
      const dx = x - space.x;
      const dz = z - space.z;
      const distance = Math.sqrt(dx * dx + dz * dz);
      if (distance > FLATTEN) continue;
      const weight = 1 - distance / FLATTEN;
      const smooth = weight * weight * (3 - 2 * weight);
      weights += smooth;
      heights += space.y * smooth;
      strongest = Math.max(strongest, smooth);
    }
    if (weights <= 0) return base;
    return base * (1 - strongest) + (heights / weights) * strongest;
  }

  /** How much the board has levelled a point, from 0 to 1: the road it cuts. */
  levelled(x: number, z: number) {
    let strongest = 0;
    for (const space of this.flats) {
      const distance = Math.hypot(x - space.x, z - space.z);
      if (distance > FLATTEN) continue;
      const weight = 1 - distance / FLATTEN;
      strongest = Math.max(strongest, weight * weight * (3 - 2 * weight));
    }
    return strongest;
  }

  /** How steeply the ground falls at a point, for sheer rock and for scatter. */
  slope(x: number, z: number) {
    const step = 0.7;
    const dx = this.height(x + step, z) - this.height(x - step, z);
    const dz = this.height(x, z + step) - this.height(x, z - step);
    return Math.sqrt(dx * dx + dz * dz) / (2 * step);
  }

  /** Nothing is planted below this; the sea is already drawn by the stage. */
  get shoreline() {
    return this.waterY + 0.35;
  }

  /**
   * The land mesh: one displaced grid shaded by height and steepness, so
   * each board's ground reads without a single texture.
   */
  build(): THREE.Mesh {
    const { cx, cz, halfW, halfD } = this.extent;
    const width = (halfW + 13) * 2;
    const depth = (halfD + 13) * 2;
    // About the same ground per cell on any size of board: 170 across the
    // boards as first drawn, more on the bigger worlds, within reason.
    // Frostline is a nearly flat city slab. The mountain grid was sending
    // over 230k triangles through every street-level frame for centimetres
    // of height variation. Keep metre-scale snow detail, while retaining the
    // dense grid on genuinely mountainous maps.
    const citySlab=this.map==='frozen'&&this.coast!==null&&this.rough<=0.1;
    const across = citySlab?Math.max(96,Math.ceil(width*.65)):Math.min(340, Math.max(170, Math.round(width * 2.2)));
    const geometry = new THREE.PlaneGeometry(width, depth, across, Math.max(24, Math.round((across * depth) / width)));
    geometry.rotateX(-Math.PI / 2);
    const position = geometry.attributes.position as THREE.BufferAttribute;
    const colors = new Float32Array(position.count * 3);
    const colour = new THREE.Color();

    for (let i = 0; i < position.count; i++) {
      const x = position.getX(i) + cx;
      const z = position.getZ(i) + cz;
      position.setY(i, this.height(x, z));
    }
    position.needsUpdate = true;

    for (let i = 0; i < position.count; i++) {
      const x = position.getX(i) + cx;
      const z = position.getZ(i) + cz;
      const y = position.getY(i);
      const steep = this.slope(x, z);
      if (this.map === 'volcano') this.paintVolcano(colour, x, z, y, steep);
      else if (this.map === 'shipwreck') this.paintCove(colour, x, z, y, steep);
      else if (this.map === 'frozen') this.paintSnow(colour, x, z, y, steep);
      else this.paintIsland(colour, x, z, y, steep);
      // A little variation across the ground, so a slope is never one flat colour.
      if (this.coast) {
        const vary = noise(x * 0.09, z * 0.09) * 0.05 + noise(x * 0.35, z * 0.35) * 0.025;
        colour.offsetHSL(vary * 0.25, vary * 0.6, vary);
      }
      colour.toArray(colors, i * 3);
    }
    geometry.setAttribute('color', new THREE.BufferAttribute(colors, 3));
    geometry.computeVertexNormals();

    const island = new THREE.Mesh(
      geometry,
      new THREE.MeshStandardMaterial({ vertexColors: true, roughness: 0.96, flatShading: true }),
    );
    island.position.set(cx, 0, cz);
    island.receiveShadow = true;
    return island;
  }

  /** Sand up the beach, grass over the low ground, and bare rock on the tops and anywhere too steep to hold soil. */
  private paintIsland(colour: THREE.Color, x: number, z: number, y: number, steep: number) {
    const beach = this.shoreline + 1.1;
    if (y < beach) colour.copy(ISLAND.sand).lerp(ISLAND.grass, Math.max(0, (y - this.shoreline) / 1.6));
    else if (y < 4.5) colour.copy(ISLAND.grass).lerp(ISLAND.deep, (y - beach) / Math.max(0.1, 4.5 - beach));
    else colour.copy(ISLAND.deep).lerp(ISLAND.rock, Math.min(1, (y - 4.5) / 4));
    if (steep > 0.75) colour.lerp(ISLAND.rock, Math.min(0.85, (steep - 0.75) * 1.2));
    // Worn ground where the board runs, so the way itself reads as a road.
    colour.lerp(ISLAND.track, this.levelled(x, z) * 0.55);
  }

  /** Black sand, ash slopes darkening to bare basalt, and a scorched bowl running red toward the middle. */
  private paintVolcano(colour: THREE.Color, x: number, z: number, y: number, steep: number) {
    const beach = this.shoreline + 0.8;
    if (y < beach) colour.copy(VOLCANO.sand).lerp(VOLCANO.ash, Math.max(0, (y - this.shoreline) / 1.2));
    else if (y < 5) colour.copy(VOLCANO.ash).lerp(VOLCANO.slope, (y - beach) / Math.max(0.1, 5 - beach));
    else colour.copy(VOLCANO.slope).lerp(VOLCANO.basalt, Math.min(1, (y - 5) / 4));
    if (steep > 0.7) colour.lerp(VOLCANO.basalt, Math.min(0.9, (steep - 0.7) * 1.4));
    if (this.crater) {
      const inside = 1 - Math.hypot(x - this.crater.x, z - this.crater.z) / this.crater.radius;
      if (inside > 0) colour.lerp(VOLCANO.scorch, Math.min(1, inside * 2.2));
      if (inside > 0.45) colour.lerp(VOLCANO.ember, Math.min(0.75, (inside - 0.45) * 2.2));
    }
    colour.lerp(VOLCANO.track, this.levelled(x, z) * 0.6);
  }

  /** Wet sand at the waterline, dry sand above, tufts of dune grass on the rises and grey rock on the headlands. */
  private paintCove(colour: THREE.Color, x: number, z: number, y: number, steep: number) {
    const wet = this.shoreline + 0.9;
    if (y < wet) colour.copy(COVE.wet).lerp(COVE.sand, Math.max(0, (y - this.shoreline) / 0.9));
    else if (y < 1.4) colour.copy(COVE.sand);
    else colour.copy(COVE.sand).lerp(COVE.dune, Math.min(0.8, (y - 1.4) / 1.2));
    if (y > 2.6 || steep > 0.7) colour.lerp(COVE.rock, Math.min(0.9, Math.max((y - 2.6) / 1.5, (steep - 0.7) * 1.5)));
    colour.lerp(COVE.track, this.levelled(x, z) * 0.5);
  }

  /** Grey shingle and blue ice at the waterline, snow above, whiter with height, and bare rock where it is too steep to settle. */
  private paintSnow(colour: THREE.Color, x: number, z: number, y: number, steep: number) {
    const shore = this.shoreline + 0.5;
    if (y < shore) colour.copy(SNOW.shingle).lerp(SNOW.ice, Math.max(0, (y - this.shoreline) / 0.5));
    else if (y < shore + 0.8) colour.copy(SNOW.ice).lerp(SNOW.low, (y - shore) / 0.8);
    else colour.copy(SNOW.low).lerp(SNOW.high, Math.min(1, (y - shore - 0.8) / 9));
    if (steep > 0.55) colour.lerp(SNOW.rock, Math.min(0.85, (steep - 0.55) * 1.3));
    colour.lerp(SNOW.track, this.levelled(x, z) * 0.55);
  }
}

const SNOW = {
  shingle: new THREE.Color('#8b96a0'),
  ice: new THREE.Color('#b9dcef'),
  low: new THREE.Color('#dbe6ee'),
  high: new THREE.Color('#ffffff'),
  rock: new THREE.Color('#6b7480'),
  track: new THREE.Color('#aebdca'),
};

const COVE = {
  wet: new THREE.Color('#c4a874'),
  sand: new THREE.Color('#f0dcaa'),
  dune: new THREE.Color('#b5b26b'),
  rock: new THREE.Color('#9b9183'),
  track: new THREE.Color('#d7bd88'),
};

const ISLAND = {
  sand: new THREE.Color('#efd9a0'),
  grass: new THREE.Color('#78c266'),
  deep: new THREE.Color('#3f8f4c'),
  rock: new THREE.Color('#8d8477'),
  track: new THREE.Color('#cdae7d'),
};

const VOLCANO = {
  sand: new THREE.Color('#3a3533'),
  ash: new THREE.Color('#7d746c'),
  slope: new THREE.Color('#4a403b'),
  basalt: new THREE.Color('#2a2321'),
  scorch: new THREE.Color('#4b2219'),
  ember: new THREE.Color('#b8400f'),
  track: new THREE.Color('#9a8a7b'),
};

/** How far a point is from a line between two others. */
function toSegment(x: number, z: number, [ax, az]: [number, number], [bx, bz]: [number, number]) {
  const dx = bx - ax;
  const dz = bz - az;
  const t = Math.max(0, Math.min(1, ((x - ax) * dx + (z - az) * dz) / (dx * dx + dz * dz || 1)));
  return Math.hypot(x - (ax + dx * t), z - (az + dz * t));
}

/** Smooth value noise from -1 to 1, the same everywhere for the same point. */
function noise(x: number, z: number) {
  const hash = (i: number, j: number) => {
    const n = Math.sin(i * 127.1 + j * 311.7) * 43758.5453;
    return (n - Math.floor(n)) * 2 - 1;
  };
  const i = Math.floor(x);
  const j = Math.floor(z);
  const fx = x - i;
  const fz = z - j;
  const u = fx * fx * (3 - 2 * fx);
  const v = fz * fz * (3 - 2 * fz);
  const top = hash(i, j) * (1 - u) + hash(i + 1, j) * u;
  const bottom = hash(i, j + 1) * (1 - u) + hash(i + 1, j + 1) * u;
  return top * (1 - v) + bottom * v;
}
