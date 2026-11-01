import * as THREE from 'three';
import { place, type Placement } from './models';
import type { Extent, Terrain } from './terrain';
import { WATER_Y } from '../stage';
import type { BoardSpace, MapKey } from '../protocol';

/** What a board's dressing needs from the scene it is dressing. */
export type Dresser = {
  group: THREE.Group;
  terrain: Terrain;
  extent: Extent;
  spaces: BoardSpace[];
  /** Ground to leave bare, such as a shop and the view into it. */
  clearings: { x: number; z: number; r: number }[];
  /** How much scatter to place, cut down on weaker machines. */
  detail: number;
  /** How much more ground and shore the board has than it was dressed for,
   * so a bigger world gets as much cover across it and along its edge. */
  area: number;
  shore: number;
  /** Shared by every pool of lava, so one pulse lights them all. */
  lava: THREE.MeshStandardMaterial;
  /** Something to move every frame, such as smoke or a flickering glow. */
  animate: (step: (time: number) => void) => void;
  track: <T extends THREE.BufferGeometry>(geometry: T) => T;
  trackMaterial: <T extends THREE.Material>(material: T) => T;
};

/** The helpers every board's dressing leans on, with a fixed seed so a board always looks the same. */
function kit(d: Dresser) {
  const { cx, cz, halfW, halfD } = d.extent;
  const ground = (x: number, z: number) => d.terrain.height(x, z);
  let seed = 1337;
  const random = () => {
    seed = (seed * 1664525 + 1013904223) % 4294967296;
    return seed / 4294967296;
  };
  return {
    cx,
    cz,
    halfW,
    halfD,
    ground,
    random,
    pick: <T,>(list: T[]) => list[Math.floor(random() * list.length)],
    /** Nothing is planted in the sea, on a cliff, or on top of the path. */
    plantable: (x: number, z: number) => ground(x, z) > d.terrain.shoreline + 0.15 && d.terrain.slope(x, z) < 0.85,
    clearOfPath: (x: number, z: number, room: number) =>
      d.spaces.every((space) => (space.x - x) ** 2 + (space.z - z) ** 2 > room * room) &&
      d.clearings.every((c) => (c.x - x) ** 2 + (c.z - z) ** 2 > (c.r + room * 0.5) ** 2),
    onLand: (x: number, z: number, margin: number) =>
      (x - cx) ** 2 / (halfW + margin) ** 2 + (z - cz) ** 2 / (halfD + margin) ** 2 < 1,
    put: (name: string, at: Placement) => void place(d.group, name, at),
  };
}

export const DRESSINGS: Record<MapKey, (d: Dresser) => void> = {
  island: dressIsland,
  volcano: dressVolcano,
  shipwreck: dressShipwreck,
  frozen: dressFrozen,
};

/** The spaces walked by following the first way out of each, from the start back round to it. */
export function lapOf(spaces: BoardSpace[]) {
  const lap = new Set<number>();
  let at = spaces[0];
  while (at && !lap.has(at.id)) {
    lap.add(at.id);
    at = spaces[at.next[0]];
  }
  return lap;
}

/**
 * On Shipwreck Cove, the spaces laid over timber: the far half of the lap,
 * which runs along the beached hulls, and the gangway through the galleon.
 */
export function onDeck(spaces: BoardSpace[]) {
  const lap = lapOf(spaces);
  return new Set(spaces.filter((space) => !lap.has(space.id) || space.z < 0).map((space) => space.id));
}

/**
 * Dresses the island from the Kenney kits: palms along the shore, woods and
 * flowers inland, a camp, a watchtower and a dock, and ships out at sea.
 */
function dressIsland(d: Dresser) {
  const { cx, cz, halfW, halfD, ground, random, pick, plantable, clearOfPath, onLand, put } = kit(d);
  const terrain = d.terrain;

  // Palms ring the shore, leaning out to sea.
  for (let i = 0; i < Math.round(34 * d.detail * d.shore); i++) {
    const angle = (i / 34) * Math.PI * 2 + random() * 0.12;
    const reach = 1.25 + random() * 0.6;
    const x = cx + Math.cos(angle) * (halfW + reach);
    const z = cz + Math.sin(angle) * (halfD + reach);
    if (!clearOfPath(x, z, 1.6) || !plantable(x, z)) continue;
    put(pick(['palm-bend', 'palm-detailed-bend', 'palm-straight']), {
      x, z, y: ground(x, z), scale: 0.55 + random() * 0.2, yaw: -angle + Math.PI / 2,
    });
  }
  // The island in bands: beach, woods on the low ground, pines up the
  // hillsides and bare rock on the tops, so height reads from the cover.
  for (let i = 0; i < Math.round(520 * d.detail * d.area); i++) {
    const x = cx + (random() * 2 - 1) * halfW;
    const z = cz + (random() * 2 - 1) * halfD;
    if (!onLand(x, z, 0.5) || !clearOfPath(x, z, 1.5)) continue;
    const y = ground(x, z);
    if (y < terrain.shoreline + 0.15) continue;
    const steep = terrain.slope(x, z);
    if (steep > 1.5) continue;
    const roll = random();
    const yaw = random() * 6;
    if (y > 7 || steep > 1.15) {
      // The tops: rock, a few hardy pines and the odd standing stone.
      if (roll < 0.52) put(pick(['rock_tallA', 'rock_tallC', 'rock_tallF', 'rock_largeB']), { x, z, y, scale: 1.4 + random(), yaw });
      else if (roll < 0.82) put(pick(['tree_pineSmallB', 'tree_pineRoundC']), { x, z, y, scale: 1.3 + random() * 0.5, yaw });
      else put(pick(['stone_largeB', 'stone_smallG']), { x, z, y, scale: 1.5, yaw });
    } else if (y > 2.2 || steep > 0.45) {
      // The slopes: pine forest, thinning into boulders.
      if (roll < 0.62) put(pick(['tree_pineTallA', 'tree_pineTallB', 'tree_pineRoundA', 'tree_pineRoundC']), { x, z, y, scale: 1.5 + random() * 0.7, yaw });
      else if (roll < 0.78) put(pick(['tree_tall', 'tree_detailed', 'tree_thin']), { x, z, y, scale: 1.5 + random() * 0.5, yaw });
      else if (roll < 0.9) put(pick(['rock_largeB', 'rock_largeD', 'rock_tallC']), { x, z, y, scale: 1.3 + random() * 0.6, yaw });
      else put(pick(['plant_bushLarge', 'grass_leafs']), { x, z, y, scale: 2, yaw });
    } else {
      // The low ground: the mixed woods the board runs through.
      if (roll < 0.3) put(pick(['tree_oak', 'tree_palmTall', 'tree_palmBend', 'tree_default']), { x, z, y, scale: 1.7 + random() * 0.6, yaw });
      else if (roll < 0.5) put(pick(['plant_bush', 'plant_bushDetailed', 'plant_bushLarge']), { x, z, y, scale: 2.2 + random(), yaw });
      else if (roll < 0.68) put(pick(['flower_redA', 'flower_yellowA', 'flowers']), { x, z, y, scale: 1.6, yaw });
      else if (roll < 0.78) put(pick(['mushroom_redGroup', 'mushrooms']), { x, z, y, scale: 1.6, yaw });
      else if (roll < 0.88) put(pick(['rock_largeA', 'rock_largeC', 'stump_round']), { x, z, y, scale: 1.6, yaw });
      else put(pick(['patch-grass-foliage', 'grass_leafs']), { x, z, y, scale: 0.35, yaw });
    }
  }

  /** The highest clear spot, for whatever should crown the island. */
  const summit = (() => {
    let best = { x: cx, z: cz, y: -Infinity };
    for (let i = 0; i < 900; i++) {
      const x = cx + (random() * 2 - 1) * halfW * 0.8;
      const z = cz + (random() * 2 - 1) * halfD * 0.8;
      const y = ground(x, z);
      if (y > best.y && clearOfPath(x, z, 2.4) && terrain.slope(x, z) < 0.6) best = { x, z, y };
    }
    return best;
  })();
  if (summit.y > 3) {
    // A broken tower on the peak, with a pennant and a few steps up to it.
    put('tower-square-base', { x: summit.x, z: summit.z, y: summit.y, scale: 1.5 });
    put('tower-square-mid-windows', { x: summit.x, z: summit.z, y: summit.y + 1.42, scale: 1.5 });
    put('tower-square-top-roof-high', { x: summit.x, z: summit.z, y: summit.y + 2.84, scale: 1.5 });
    put('flag-pennant', { x: summit.x + 1.3, z: summit.z + 0.9, y: summit.y, scale: 1.2, yaw: 0.6 });
    put('stairs-stone', { x: summit.x - 1.6, z: summit.z + 0.4, y: summit.y - 0.3, scale: 1.3, yaw: Math.PI / 2 });
    put('siege-catapult', { x: summit.x + 2.2, z: summit.z - 1.4, y: ground(summit.x + 2.2, summit.z - 1.4), scale: 1.1, yaw: 2.3 });
  }

  const landmark = landmarks(d);
  landmark(Math.PI * 0.8, 1.2, 'tower-watch', 0.9);
  landmark(Math.PI * 1.35, 1.0, 'tent', 1.6, (x, z) => {
    put('campfire-pit', { x: x + 1.2, z: z + 0.6, y: ground(x + 1.2, z + 0.6), scale: 1.4 });
    put('bedroll', { x: x - 1.1, z: z + 0.9, y: ground(x - 1.1, z + 0.9), scale: 1.4, yaw: 0.6 });
    put('box-large', { x: x + 0.3, z: z - 1.3, y: ground(x + 0.3, z - 1.3), scale: 1.2, yaw: 0.9 });
  });
  landmark(Math.PI * 0.15, 0.6, 'barrel', 0.45, (x, z) => {
    put('crate', { x: x + 0.8, z: z - 0.3, y: ground(x + 0.8, z - 0.3), scale: 0.55, yaw: 0.4 });
    put('signpost', { x: x - 0.9, z: z + 0.4, y: ground(x - 0.9, z + 0.4), scale: 1.2, yaw: 1.1 });
  });
  // A small graveyard in a hollow, fenced, with a lamp burning.
  landmark(Math.PI * 1.08, 2.2, 'crypt-small', 1.3, (x, z) => {
    for (const [dx, dz, name] of [
      [1.5, 0.4, 'gravestone-cross'],
      [2.3, -0.7, 'gravestone-round'],
      [1.1, -1.5, 'cross-wood'],
      [2.9, 0.8, 'iron-fence'],
    ] as [number, number, string][]) {
      put(name, { x: x + dx, z: z + dz, y: ground(x + dx, z + dz), scale: 1.2, yaw: random() * 6 });
    }
    put('lightpost-single', { x: x - 1.4, z: z - 0.6, y: ground(x - 1.4, z - 0.6), scale: 1.2 });
  });

  cliffs(d, random, {});

  // A dock and a rowboat on the shore, and ships riding at anchor.
  const dockAngle = Math.PI * 1.75;
  const dockX = cx + Math.cos(dockAngle) * (halfW + 3.2);
  const dockZ = cz + Math.sin(dockAngle) * (halfD + 3.2);
  put('structure-platform-dock', { x: dockX, z: dockZ, y: WATER_Y + 0.35, scale: 1.1, yaw: -dockAngle + Math.PI / 2 });
  put('boat-row-small', { x: dockX + 2.4, z: dockZ + 0.6, y: WATER_Y + 0.2, scale: 0.75, yaw: -dockAngle });
  put('ship-pirate-medium', { x: cx - halfW - 11, z: cz - halfD * 0.4, y: WATER_Y + 0.1, scale: 0.55, yaw: 0.5 });
  put('ship-wreck', { x: cx + halfW + 10, z: cz + halfD * 0.7, y: WATER_Y - 0.2, scale: 0.5, yaw: -2.2 });
  put('rocks-sand-a', { x: cx + halfW * 0.4, z: cz - halfD - 5.5, y: WATER_Y + 0.1, scale: 0.45, yaw: 1.2 });
  put('rocks-b', { x: cx - halfW * 0.6, z: cz + halfD + 5, y: WATER_Y + 0.05, scale: 0.4, yaw: 2.4 });
}

/** Landmarks on whatever open ground is left near the edge, tried at a few angles until one fits. */
function landmarks(d: Dresser) {
  const { cx, cz, halfW, halfD, ground, plantable, clearOfPath, put } = kit(d);
  return (angle: number, inset: number, name: string, scale: number, extra?: (x: number, z: number) => void, paint?: Placement['paint']) => {
    for (let tries = 0; tries < 14; tries++) {
      const a = angle + (tries % 2 ? 1 : -1) * tries * 0.08;
      const x = cx + Math.cos(a) * (halfW - inset);
      const z = cz + Math.sin(a) * (halfD - inset);
      if (!clearOfPath(x, z, 2.2) || !plantable(x, z)) continue;
      put(name, { x, z, y: ground(x, z), scale, yaw: -a - Math.PI / 2, paint });
      extra?.(x, z);
      return;
    }
  };
}

/** Cliffs where the ground falls away hardest, to break up the slopes. */
function cliffs(d: Dresser, random: () => number, paint: Placement['paint'], count = 160) {
  const { cx, cz, halfW, halfD, ground, clearOfPath, onLand, put } = kit(d);
  const names = ['cliff_rock', 'cliff_large_rock', 'cliff_blockSlope_rock', 'cliff_half_rock'];
  for (let i = 0; i < Math.round(count * d.detail * d.area); i++) {
    const x = cx + (random() * 2 - 1) * halfW;
    const z = cz + (random() * 2 - 1) * halfD;
    if (!onLand(x, z, 0.2) || !clearOfPath(x, z, 4)) continue;
    const y = ground(x, z);
    if (y < d.terrain.shoreline + 0.4 || d.terrain.slope(x, z) < 1.15) continue;
    put(names[Math.floor(random() * names.length)], { x, z, y: y - 0.6, scale: 1.6 + random() * 0.8, yaw: random() * 6, paint });
  }
}

/**
 * A patch laid over the ground and bent to follow it, with a ragged edge,
 * so a pool or a sheet of ice sits on a slope instead of cutting into it.
 */
export function groundPatch(d: Dresser, x: number, z: number, radius: number, material: THREE.Material, lift = 0.06) {
  const geometry = d.track(new THREE.RingGeometry(0, radius, 28, 4));
  geometry.rotateX(-Math.PI / 2);
  const position = geometry.attributes.position as THREE.BufferAttribute;
  const base = d.terrain.height(x, z);
  for (let i = 0; i < position.count; i++) {
    const px = position.getX(i);
    const pz = position.getZ(i);
    const angle = Math.atan2(pz, px);
    const ragged = 1 + Math.sin(angle * 3 + x) * 0.12 + Math.sin(angle * 7 + z) * 0.06;
    const vx = px * ragged;
    const vz = pz * ragged;
    position.setXYZ(i, vx, d.terrain.height(x + vx, z + vz) - base + lift, vz);
  }
  geometry.computeVertexNormals();
  const patch = new THREE.Mesh(geometry, material);
  patch.position.set(x, base, z);
  patch.receiveShadow = true;
  d.group.add(patch);
  return patch;
}

/** Smoke rolling up from a point, thinning as it climbs and leaning off downwind. */
export function smokePlume(d: Dresser, from: { x: number; z: number }, random: () => number) {
  const puff = d.track(new THREE.IcosahedronGeometry(1, 1));
  const base = d.terrain.height(from.x, from.z);
  const count = Math.round(16 * Math.max(0.5, d.detail));
  const puffs = Array.from({ length: count }, (_, index) => {
    const material = d.trackMaterial(
      new THREE.MeshStandardMaterial({ color: '#6b6460', roughness: 1, transparent: true, opacity: 0.4, depthWrite: false, flatShading: true }),
    );
    const mesh = new THREE.Mesh(puff, material);
    d.group.add(mesh);
    return { mesh, material, offset: index / count, drift: random() * Math.PI * 2 };
  });
  d.animate((time) => {
    for (const one of puffs) {
      const t = (time * 0.06 + one.offset) % 1;
      one.mesh.position.set(
        from.x + Math.cos(one.drift) * t * 1.5 + t * t * 7,
        base + 0.8 + t * 20,
        from.z + Math.sin(one.drift) * t * 1.5 - t * t * 3,
      );
      one.mesh.scale.setScalar(0.6 + t * 2.4);
      one.mesh.rotation.set(t * 2, one.drift + t, 0);
      one.material.opacity = 0.5 * Math.min(1, t * 6) * (1 - t);
    }
  });
}

/** A pool of glowing lava lying on the ground. */
export function lavaPool(d: Dresser, x: number, z: number, radius: number) {
  let crust = crusts.get(d);
  if (!crust) {
    crust = d.trackMaterial(new THREE.MeshStandardMaterial({ color: '#1f1a19', roughness: 1, flatShading: true }));
    crusts.set(d, crust);
  }
  groundPatch(d, x, z, radius * 1.3, crust, 0.04);
  return groundPatch(d, x, z, radius, d.lava, 0.08);
}

const crusts = new WeakMap<Dresser, THREE.Material>();

/** A glassy sheet of ice lying on the ground. */
export function iceSheet(d: Dresser, x: number, z: number, radius: number) {
  let ice = ices.get(d);
  if (!ice) {
    ice = d.trackMaterial(new THREE.MeshStandardMaterial({ color: '#cbeaf8', roughness: 0.08, metalness: 0.15, flatShading: true }));
    ices.set(d, ice);
  }
  return groundPatch(d, x, z, radius, ice, 0.05);
}

const ices = new WeakMap<Dresser, THREE.Material>();

/** Basalt and ash in place of the kits' dirt and grass. */
export const BASALT = { dirt: '#3e3532', grass: '#5a514c', _defaultMat: '#4a413d', stone: '#4d4440', stoneDark: '#2e2724' };
/** Trees with the leaves burnt off and the bark charred. */
export const CHARRED = { leafsGreen: null, leafsDark: null, woodBark: '#3d302b', woodBarkDark: '#342824', woodInner: '#6a4532' };

/**
 * The volcano: a black beach, ash slopes with burnt-out trees, basalt
 * crags up the cone, and a crater with lava lying in it, smoking.
 */
function dressVolcano(d: Dresser) {
  const { cx, cz, halfW, halfD, ground, random, pick, plantable, clearOfPath, onLand, put } = kit(d);
  const terrain = d.terrain;
  const crater = terrain.crater ?? { x: cx, z: cz, radius: Math.min(halfW, halfD) * 0.45 };

  // Lava in the bottom of the crater, in pools either side of the crossing.
  const pools: { x: number; z: number; r: number }[] = [];
  for (let i = 0; i < 260 && pools.length < 5; i++) {
    const angle = random() * Math.PI * 2;
    const out = Math.sqrt(random()) * crater.radius * 0.7;
    const x = crater.x + Math.cos(angle) * out;
    const z = crater.z + Math.sin(angle) * out;
    const r = 1.1 + random() * 1.3;
    if (!clearOfPath(x, z, r + 1.3)) continue;
    if (pools.some((pool) => Math.hypot(pool.x - x, pool.z - z) < pool.r + r + 0.6)) continue;
    pools.push({ x, z, r });
    lavaPool(d, x, z, r);
  }
  const glow = new THREE.PointLight('#ff6a1f', 60, crater.radius * 3.2, 1.6);
  glow.position.set(crater.x, ground(crater.x, crater.z) + 2.5, crater.z);
  d.group.add(glow);
  d.animate((time) => {
    glow.intensity = 52 + Math.sin(time * 2.3) * 8 + Math.sin(time * 5.1) * 4;
  });

  smokePlume(d, pools[0] ?? { x: crater.x, z: crater.z }, random);

  // Basalt crags along the crater rim.
  for (let i = 0; i < Math.round(30 * d.detail * d.shore); i++) {
    const angle = (i / 30) * Math.PI * 2 + random() * 0.15;
    const out = crater.radius * (0.95 + random() * 0.3);
    const x = crater.x + Math.cos(angle) * out;
    const z = crater.z + Math.sin(angle) * out;
    if (!clearOfPath(x, z, 1.8)) continue;
    put(pick(['rock_tallA', 'rock_tallC', 'rock_tallF', 'rock_largeB']), { x, z, y: ground(x, z) - 0.2, scale: 1.4 + random() * 1.2, yaw: random() * 6, paint: BASALT });
  }

  // The cone in bands: black sand and driftwood, ash fields with dead trees,
  // and bare crags higher up. Sparse, since nothing much grows here.
  for (let i = 0; i < Math.round(340 * d.detail * d.area); i++) {
    const x = cx + (random() * 2 - 1) * halfW;
    const z = cz + (random() * 2 - 1) * halfD;
    if (!onLand(x, z, 0.5) || !clearOfPath(x, z, 1.5)) continue;
    if (Math.hypot(x - crater.x, z - crater.z) < crater.radius * 0.9) continue;
    const y = ground(x, z);
    if (y < terrain.shoreline + 0.15) continue;
    const steep = terrain.slope(x, z);
    if (steep > 1.5) continue;
    const roll = random();
    const yaw = random() * 6;
    if (y > 6 || steep > 1.1) {
      if (roll < 0.6) put(pick(['rock_tallA', 'rock_tallC', 'rock_tallF', 'rock_largeD']), { x, z, y, scale: 1.3 + random(), yaw, paint: BASALT });
      else if (roll < 0.8) put(pick(['stone_largeB', 'stone_smallG']), { x, z, y, scale: 1.6, yaw, paint: BASALT });
    } else if (y > 1.6) {
      if (roll < 0.34) put(pick(['tree_thin', 'tree_tall', 'tree_default', 'tree_detailed', 'tree_pineTallA', 'tree_pineRoundC']), { x, z, y, scale: 1.5 + random() * 0.7, yaw, paint: CHARRED });
      else if (roll < 0.62) put(pick(['rock_largeA', 'rock_largeB', 'rock_largeC', 'rock_largeD']), { x, z, y, scale: 1.2 + random() * 0.8, yaw, paint: BASALT });
      else if (roll < 0.72) put('stump_round', { x, z, y, scale: 1.6, yaw, paint: CHARRED });
      else if (roll < 0.8) put('log_stack', { x, z, y, scale: 1.3, yaw, paint: CHARRED });
    } else {
      if (roll < 0.3) put(pick(['tree_palmBend', 'tree_palmTall']), { x, z, y, scale: 1.5 + random() * 0.4, yaw, paint: CHARRED });
      else if (roll < 0.55) put(pick(['rock_largeA', 'rock_largeC', 'stone_smallG']), { x, z, y, scale: 1.4, yaw, paint: BASALT });
      else if (roll < 0.62) put('stump_round', { x, z, y, scale: 1.4, yaw, paint: CHARRED });
    }
  }

  // Lava welling up in the gaps between the lap and the cone.
  let outside = 0;
  for (let i = 0; i < 400 * d.shore && outside < Math.round(7 * Math.max(0.6, d.detail) * d.shore); i++) {
    const x = cx + (random() * 2 - 1) * halfW;
    const z = cz + (random() * 2 - 1) * halfD;
    const r = 0.6 + random() * 0.7;
    if (!onLand(x, z, -1.5) || !clearOfPath(x, z, r + 1.6) || !plantable(x, z)) continue;
    if (Math.hypot(x - crater.x, z - crater.z) < crater.radius * 1.3) continue;
    if (pools.some((pool) => Math.hypot(pool.x - x, pool.z - z) < pool.r + r + 3)) continue;
    pools.push({ x, z, r });
    lavaPool(d, x, z, r);
    for (let k = 0; k < 3; k++) {
      const angle = random() * Math.PI * 2;
      const px = x + Math.cos(angle) * (r + 0.4);
      const pz = z + Math.sin(angle) * (r + 0.4);
      put('stone_smallG', { x: px, z: pz, y: ground(px, pz), scale: 1.4 + random(), yaw: random() * 6, paint: BASALT });
    }
    outside++;
  }

  cliffs(d, random, BASALT, 200);

  // A ruined shrine and an abandoned camp near the foot, and a wreck offshore.
  const landmark = landmarks(d);
  landmark(Math.PI * 0.8, 1.4, 'statue_obelisk', 1.6, (x, z) => {
    put('statue_columnDamaged', { x: x + 1.5, z: z + 0.6, y: ground(x + 1.5, z + 0.6), scale: 1.5, yaw: 0.4, paint: BASALT });
    put('statue_columnDamaged', { x: x - 1.4, z: z + 0.9, y: ground(x - 1.4, z + 0.9), scale: 1.3, yaw: 2.1, paint: BASALT });
    put('campfire_stones', { x: x + 0.2, z: z + 1.6, y: ground(x + 0.2, z + 1.6), scale: 1.4, paint: BASALT });
  }, BASALT);
  landmark(Math.PI * 1.4, 1.0, 'tent_smallOpen', 1.5, (x, z) => {
    put('campfire-pit', { x: x + 1.2, z: z + 0.6, y: ground(x + 1.2, z + 0.6), scale: 1.4 });
    put('cart', { x: x - 1.3, z: z + 0.5, y: ground(x - 1.3, z + 0.5), scale: 0.9, yaw: 1.2 });
  }, { colorRed: '#6b5a50', colorRedDark: '#4a3c35', wood: '#3a2c25' });
  landmark(Math.PI * 0.1, 0.8, 'gravestone-cross', 1.3, (x, z) => {
    put('gravestone-round', { x: x + 0.9, z: z + 0.5, y: ground(x + 0.9, z + 0.5), scale: 1.2, yaw: 0.3 });
    put('lantern', { x: x - 0.7, z: z + 0.4, y: ground(x - 0.7, z + 0.4), scale: 1.3 });
  });
  put('ship-wreck', { x: cx - halfW - 9, z: cz + halfD * 0.5, y: WATER_Y - 0.3, scale: 0.5, yaw: 0.9 });
  put('rocks-large', { x: cx + halfW + 7, z: cz - halfD * 0.3, y: WATER_Y - 0.2, scale: 0.55, yaw: 1.7, paint: { colormap: '#4a4442' } });
  put('rocks-b', { x: cx + halfW * 0.3, z: cz + halfD + 6, y: WATER_Y + 0.05, scale: 0.45, yaw: 2.4, paint: { colormap: '#4a4442' } });
}

/** Pale, dry grass for the dunes. */
const DUNE_GRASS = { grass: '#a9b86a' };

/**
 * Shipwreck Cove: hulls run aground along the far side with the lap laid on
 * planks beside them, a galleon broken across the middle, and a beach of
 * palms, rowboats and castaway camps on the near side where the tide comes in.
 */
function dressShipwreck(d: Dresser) {
  const { cx, cz, halfW, halfD, ground, random, pick, plantable, clearOfPath, onLand, put } = kit(d);
  const terrain = d.terrain;
  const spaces = d.spaces;
  const lap = lapOf(spaces);
  const deck = onDeck(spaces);
  /** Which way the board runs at a space, as a yaw. */
  const heading = (id: number) => {
    const here = spaces[id];
    const next = spaces[here.next[0]];
    return Math.atan2(next.x - here.x, next.z - here.z);
  };

  // A hull every few spaces along the far side, lying alongside the planks,
  // sunk into the sand and listing away from the path.
  const hullSide = spaces.filter((space) => lap.has(space.id) && deck.has(space.id));
  hullSide.forEach((space, index) => {
    if (index % 4 !== 1) return;
    const yaw = heading(space.id);
    const outX = space.x - cx;
    const outZ = space.z - cz;
    const length = Math.hypot(outX, outZ) || 1;
    const x = space.x + (outX / length) * 3.4;
    const z = space.z + (outZ / length) * 3.4;
    put('ship-wreck', { x, z, y: ground(x, z) - 0.7, scale: 0.75 + random() * 0.2, yaw: yaw + Math.PI / 2 + (random() - 0.5) * 0.4 });
  });

  // The galleon across the middle: two broken halves either side of the
  // gangway, and its cargo strewn about.
  const gangway = spaces.filter((space) => !lap.has(space.id));
  if (gangway.length) {
    const middle = gangway[Math.floor(gangway.length / 2)];
    const yaw = heading(middle.id);
    const side = { x: Math.cos(yaw), z: -Math.sin(yaw) };
    put('ship-wreck', { x: middle.x + side.x * 3.6, z: middle.z + side.z * 3.6, y: ground(middle.x, middle.z) - 0.5, scale: 1.25, yaw: yaw + 0.25 });
    put('ship-wreck', { x: middle.x - side.x * 3.8, z: middle.z - side.z * 3.8, y: ground(middle.x, middle.z) - 0.8, scale: 1.1, yaw: yaw + Math.PI - 0.35 });
    for (const space of gangway) {
      if (random() < 0.45) continue;
      const flip = random() < 0.5 ? 1 : -1;
      const x = space.x + side.x * flip * (1.4 + random() * 0.6);
      const z = space.z + side.z * flip * (1.4 + random() * 0.6);
      if (!clearOfPath(x, z, 1.1)) continue;
      put(pick(['barrel', 'crate', 'resource-planks', 'bottle', 'cannon']), { x, z, y: ground(x, z), scale: 0.6 + random() * 0.3, yaw: random() * 6 });
    }
  }

  // Timber, rigging and stores scattered among the hulls.
  for (let i = 0; i < Math.round(90 * d.detail * d.area); i++) {
    const x = cx + (random() * 2 - 1) * halfW;
    const z = cz - random() * halfD;
    if (!onLand(x, z, 0.3) || !clearOfPath(x, z, 1.3) || !plantable(x, z)) continue;
    const roll = random();
    const y = ground(x, z);
    if (roll < 0.35) put(pick(['barrel', 'crate', 'box-large']), { x, z, y, scale: 0.55 + random() * 0.3, yaw: random() * 6 });
    else if (roll < 0.55) put('resource-planks', { x, z, y, scale: 0.8, yaw: random() * 6 });
    else if (roll < 0.65) put('cannon', { x, z, y, scale: 0.6, yaw: random() * 6 });
    else if (roll < 0.8) put(pick(['rocks-sand-a', 'rock-sand-b']), { x, z, y, scale: 0.3 + random() * 0.2, yaw: random() * 6 });
    else put('grass_leafs', { x, z, y, scale: 1.4, yaw: random() * 6, paint: DUNE_GRASS });
  }

  // The beach: palms leaning out over the near shore, dune grass and the odd
  // rock behind it, and a few hardy trees on the rises.
  for (let i = 0; i < Math.round(26 * d.detail * d.shore); i++) {
    const angle = Math.PI * (0.05 + (i / 26) * 0.9) + random() * 0.08;
    const reach = 1.1 + random() * 0.7;
    const x = cx + Math.cos(angle) * (halfW + reach);
    const z = cz + Math.sin(angle) * (halfD + reach);
    if (!clearOfPath(x, z, 1.6) || !plantable(x, z)) continue;
    put(pick(['palm-bend', 'palm-detailed-bend', 'palm-straight']), { x, z, y: ground(x, z), scale: 0.55 + random() * 0.2, yaw: -angle + Math.PI / 2 });
  }
  for (let i = 0; i < Math.round(220 * d.detail * d.area); i++) {
    const x = cx + (random() * 2 - 1) * halfW;
    const z = cz + (random() * 2 - 1) * halfD;
    if (!onLand(x, z, 0.5) || !clearOfPath(x, z, 1.5)) continue;
    const y = ground(x, z);
    if (y < terrain.shoreline + 0.4) continue;
    const steep = terrain.slope(x, z);
    if (steep > 1.3) continue;
    const roll = random();
    const yaw = random() * 6;
    if (y > 2.6 || steep > 0.8) {
      if (roll < 0.6) put(pick(['rock_largeA', 'rock_largeC', 'rock_tallC', 'rocks-large']), { x, z, y, scale: 1.2 + random() * 0.6, yaw });
    } else if (z > cz) {
      if (roll < 0.22) put(pick(['tree_palmTall', 'tree_palmBend']), { x, z, y, scale: 1.6 + random() * 0.5, yaw });
      else if (roll < 0.48) put('grass_leafs', { x, z, y, scale: 1.6, yaw, paint: DUNE_GRASS });
      else if (roll < 0.56) put(pick(['rocks-sand-a', 'rock-sand-b']), { x, z, y, scale: 0.35, yaw });
      else if (roll < 0.62) put('plant_bush', { x, z, y, scale: 2, yaw, paint: DUNE_GRASS });
    } else if (roll < 0.12) {
      put(pick(['tree_palmBend', 'plant_bushLarge']), { x, z, y, scale: 1.6, yaw });
    }
  }

  // Rowboats pulled up on the sand, and a camp the castaways made.
  const landmark = landmarks(d);
  landmark(Math.PI * 0.35, 0.4, 'boat-row-small', 0.8);
  landmark(Math.PI * 0.68, 0.4, 'boat-row-small', 0.8, (x, z) => {
    put('bottle', { x: x + 1.1, z: z - 0.4, y: ground(x + 1.1, z - 0.4), scale: 1.2, yaw: 1.3 });
  });
  landmark(Math.PI * 0.52, 3.2, 'tent', 1.5, (x, z) => {
    put('campfire-pit', { x: x + 1.3, z: z + 0.5, y: ground(x + 1.3, z + 0.5), scale: 1.4 });
    put('bedroll', { x: x - 1.1, z: z + 0.9, y: ground(x - 1.1, z + 0.9), scale: 1.4, yaw: 0.6 });
    put('chest', { x: x + 0.2, z: z - 1.4, y: ground(x + 0.2, z - 1.4), scale: 0.7, yaw: 0.9 });
    put('signpost', { x: x - 1.6, z: z - 0.6, y: ground(x - 1.6, z - 0.6), scale: 1.2, yaw: 2.2 });
  });
  // A lookout and a flag on the headlands.
  landmark(Math.PI * 1.22, 2.6, 'tower-watch', 0.9);
  landmark(Math.PI * 1.8, 2.6, 'flag-pirate-high', 0.7, (x, z) => {
    put('lantern', { x: x + 0.8, z: z + 0.5, y: ground(x + 0.8, z + 0.5), scale: 1.2 });
  });

  cliffs(d, random, {}, 120);

  // A jetty off the beach and ships still afloat beyond the hulls.
  const dockAngle = Math.PI * 0.42;
  const dockX = cx + Math.cos(dockAngle) * (halfW + 3);
  const dockZ = cz + Math.sin(dockAngle) * (halfD + 3);
  put('structure-platform-dock', { x: dockX, z: dockZ, y: WATER_Y + 0.35, scale: 1.1, yaw: -dockAngle + Math.PI / 2 });
  put('ship-pirate-medium', { x: cx + halfW + 10, z: cz + halfD * 0.6, y: WATER_Y + 0.1, scale: 0.6, yaw: -2.4 });
  put('ship-wreck', { x: cx - halfW - 8, z: cz - halfD * 0.2, y: WATER_Y - 0.6, scale: 0.7, yaw: 1.1 });
  put('ship-wreck', { x: cx + halfW * 0.2, z: cz - halfD - 7, y: WATER_Y - 0.8, scale: 0.65, yaw: 2.6 });
  put('rocks-sand-a', { x: cx - halfW * 0.5, z: cz + halfD + 5, y: WATER_Y + 0.1, scale: 0.45, yaw: 1.2 });
}

/** Grey stone with snow lying where the kits had grass. */
export const SNOWCAPPED = { dirt: '#7c8590', grass: '#f2f6f9', _defaultMat: '#8a939c', stone: '#8a939c', stoneDark: '#5f6870' };
/** Dark firs, a little frosted. */
export const FIR = { leafsDark: '#3d6658', woodBarkDark: '#5a4636', leafsGreen: '#4f7466', woodBark: '#6a523f' };

/**
 * Frozen Peak, kept sparse: a few firs low down, snowcapped boulders and
 * crags higher up, a frozen tarn, a base camp at the foot, a flag on the
 * summit, and snow falling over all of it.
 */
function dressFrozen(d: Dresser) {
  const { cx, cz, halfW, halfD, ground, random, pick, plantable, clearOfPath, onLand, put } = kit(d);
  const terrain = d.terrain;

  for (let i = 0; i < Math.round(260 * d.detail * d.area); i++) {
    const x = cx + (random() * 2 - 1) * halfW;
    const z = cz + (random() * 2 - 1) * halfD;
    if (!onLand(x, z, 0.5) || !clearOfPath(x, z, 1.6)) continue;
    const y = ground(x, z);
    if (y < terrain.shoreline + 0.5) continue;
    const steep = terrain.slope(x, z);
    if (steep > 1.4) continue;
    const roll = random();
    const yaw = random() * 6;
    if (y > 9 || steep > 1) {
      if (roll < 0.35) put(pick(['rock_tallA', 'rock_tallC', 'rock_tallF', 'rock_largeB']), { x, z, y, scale: 1.3 + random(), yaw, paint: SNOWCAPPED });
    } else if (y > 4) {
      if (roll < 0.22) put(pick(['tree_pineSmallB', 'tree_pineRoundC', 'tree_pineTallA']), { x, z, y, scale: 1.2 + random() * 0.5, yaw, paint: FIR });
      else if (roll < 0.36) put(pick(['rock_largeA', 'rock_largeD', 'stone_largeB']), { x, z, y, scale: 1.2 + random() * 0.6, yaw, paint: SNOWCAPPED });
    } else {
      if (roll < 0.3) put(pick(['tree_pineTallA', 'tree_pineTallB', 'tree_pineRoundA', 'tree_pineRoundC']), { x, z, y, scale: 1.5 + random() * 0.6, yaw, paint: FIR });
      else if (roll < 0.4) put(pick(['rock_largeA', 'rock_largeC', 'stone_smallG']), { x, z, y, scale: 1.3, yaw, paint: SNOWCAPPED });
      else if (roll < 0.44) put('stump_round', { x, z, y, scale: 1.4, yaw });
    }
  }

  cliffs(d, random, SNOWCAPPED, 60);

  // A frozen tarn in the flattest open ground, and ice in a few hollows.
  let tarns = 0;
  for (let i = 0; i < 500 && tarns < 4; i++) {
    const x = cx + (random() * 2 - 1) * halfW;
    const z = cz + (random() * 2 - 1) * halfD;
    const r = tarns === 0 ? 3.2 : 1.2 + random();
    if (!onLand(x, z, -1) || !clearOfPath(x, z, r + 1.4) || !plantable(x, z) || terrain.slope(x, z) > 0.35) continue;
    iceSheet(d, x, z, r);
    tarns++;
  }

  // The highest gated space carries a flag and a cairn.
  const summit = d.spaces.filter((space) => space.gated).sort((a, b) => ground(b.x, b.z) - ground(a.x, a.z))[0];
  if (summit) {
    for (const [dx, dz, name, scale] of [
      [1.6, 0.6, 'flag', 1.4],
      [-1.4, 1.0, 'statue_obelisk', 0.9],
      [0.4, -1.7, 'stone_largeB', 1.2],
    ] as [number, number, string, number][]) {
      const x = summit.x + dx;
      const z = summit.z + dz;
      put(name, { x, z, y: ground(x, z), scale, yaw: random() * 6, paint: SNOWCAPPED });
    }
  }

  const landmark = landmarks(d);
  landmark(Math.PI * 0.6, 1.2, 'tent', 1.5, (x, z) => {
    put('campfire-pit', { x: x + 1.3, z: z + 0.6, y: ground(x + 1.3, z + 0.6), scale: 1.4 });
    put('log_stack', { x: x - 1.3, z: z + 0.6, y: ground(x - 1.3, z + 0.6), scale: 1.3, yaw: 0.8 });
    put('cart', { x: x + 0.4, z: z - 1.6, y: ground(x + 0.4, z - 1.6), scale: 0.9, yaw: 1.9 });
  });
  landmark(Math.PI * 0.45, 1.6, 'tent_smallOpen', 1.4, undefined, { colorRed: '#2f6f9f', colorRedDark: '#24577d' });
  landmark(Math.PI * 1.3, 1.0, 'signpost', 1.3);

  snowfall(d, random);
}

/** Snow drifting down over the board, wrapping round as it reaches the ground. */
export function snowfall(d: Dresser, random: () => number) {
  const { cx, cz, halfW, halfD } = d.extent;
  const count = Math.min(4500,Math.round(900 * Math.max(0.4, d.detail) * d.area));
  const flakes = new Float32Array(count * 3);
  const top = 30;
  for (let i = 0; i < count; i++) {
    flakes[i * 3] = cx + (random() * 2 - 1) * (halfW + 8);
    flakes[i * 3 + 1] = random() * top;
    flakes[i * 3 + 2] = cz + (random() * 2 - 1) * (halfD + 8);
  }
  const geometry = d.track(new THREE.BufferGeometry());
  geometry.setAttribute('position', new THREE.BufferAttribute(flakes, 3));
  const snow = new THREE.Points(
    geometry,
    d.trackMaterial(new THREE.PointsMaterial({ color: '#ffffff', size: 0.16, transparent: true, opacity: 0.85, depthWrite: false })),
  );
  snow.frustumCulled = false;
  d.group.add(snow);
  // Animate on the GPU: the enlarged city must not rewrite/upload thousands
  // of particle positions on the CPU every frame.
  const clock={value:0};
  snow.material.onBeforeCompile=shader=>{
    shader.uniforms.uSnowTime=clock;
    shader.vertexShader='uniform float uSnowTime;\n'+shader.vertexShader;
    shader.vertexShader=shader.vertexShader.replace('#include <begin_vertex>',`#include <begin_vertex>
      transformed.y=mod(position.y-uSnowTime*1.6,30.0);
      transformed.x+=sin(uSnowTime*.6+position.x*13.7+position.z*.3)*.6;`);
  };
  d.animate(time=>{clock.value=time;});
}
