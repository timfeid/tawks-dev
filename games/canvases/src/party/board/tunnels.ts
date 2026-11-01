import * as THREE from 'three';
import type { Terrain } from './terrain';
import type { BoardSpace, MapKey } from '../protocol';

/** How far below its mouths a tunnel runs once it is in. */
const DEPTH = 4.6;
/** At least this much rock over the tunnel's roof... */
const COVER = 3.8;
/** ...which is this far above the path. */
const RADIUS = 1.75;
const LIFT = 1.5;

/** Each tunnel's look: its rock, and what lights it. */
const LOOKS: Record<MapKey, { rock: string; light: string; flame: string }> = {
  island: { rock: '#5b4a3c', light: '#ffb35c', flame: '#ffcf70' },
  volcano: { rock: '#3b221b', light: '#ff5a1f', flame: '#ff7a2f' },
  shipwreck: { rock: '#5b4a3c', light: '#ffb35c', flame: '#ffcf70' },
  frozen: { rock: '#8fb3cc', light: '#7dd3fc', flame: '#c4f1ff' },
};

/** One tunnel: the space it is entered from, the spaces in it, and the one it comes out at. */
export type Tunnel = { mouth: number; inside: number[]; exit: number };

/** The tunnels on a board, each followed from the space before it to the one after. */
export function findTunnels(spaces: BoardSpace[]): Tunnel[] {
  const byId = new Map(spaces.map((space) => [space.id, space]));
  const tunnels: Tunnel[] = [];
  for (const space of spaces) {
    if (space.under) continue;
    for (const next of space.next) {
      if (!byId.get(next)?.under) continue;
      const inside: number[] = [];
      let at = next;
      while (byId.get(at)?.under && inside.length < spaces.length) {
        inside.push(at);
        at = byId.get(at)!.next[0];
      }
      tunnels.push({ mouth: space.id, inside, exit: at });
    }
  }
  return tunnels;
}

/**
 * The height of each space in a tunnel: down from its mouth, along at depth,
 * and up to where it comes out, never so near the ground above that the
 * roof breaks through it.
 */
export function tunnelHeights(tunnels: Tunnel[], spaces: BoardSpace[], terrain: Terrain) {
  const byId = new Map(spaces.map((space) => [space.id, space]));
  const heights = new Map<number, number>();
  for (const { mouth, inside, exit } of tunnels) {
    const a = byId.get(mouth)!;
    const b = byId.get(exit)!;
    const from = terrain.height(a.x, a.z);
    const to = terrain.height(b.x, b.z);
    const count = inside.length + 1;
    inside.forEach((id, i) => {
      const space = byId.get(id)!;
      const t = (i + 1) / count;
      // Full depth a space and a half in from either end.
      const dip = DEPTH * Math.min(1, (Math.min(t, 1 - t) * count) / 1.5);
      let y = from + (to - from) * t - dip;
      if (i > 0 && i < inside.length - 1) y = Math.min(y, terrain.height(space.x, space.z) - COVER - RADIUS - LIFT);
      heights.set(id, y);
    });
  }
  // Smooth the floor once, so a hill overhead does not leave a step in it.
  for (const { inside } of tunnels) {
    const raw = inside.map((id) => heights.get(id)!);
    inside.forEach((id, i) => {
      if (i === 0 || i === inside.length - 1) return;
      heights.set(id, Math.min(raw[i], (raw[i - 1] + raw[i] * 2 + raw[i + 1]) / 4));
    });
  }
  return heights;
}

type Kit = {
  group: THREE.Group;
  detail: number;
  track: <T extends THREE.BufferGeometry>(geometry: T) => T;
  trackMaterial: <T extends THREE.Material>(material: T) => T;
  animate: (step: (time: number) => void) => void;
};

/**
 * The tunnels themselves: a bore of rough rock round the path, a ring of
 * rock and a dark hole at each mouth, and lights along the walls.
 */
export function buildTunnels(kit: Kit, tunnels: Tunnel[], positions: Map<number, THREE.Vector3>, map: MapKey) {
  const look = LOOKS[map];
  const rock = kit.trackMaterial(
    new THREE.MeshStandardMaterial({ color: look.rock, roughness: 1, flatShading: true, side: THREE.DoubleSide }),
  );
  const dark = kit.trackMaterial(new THREE.MeshBasicMaterial({ color: '#050403', side: THREE.DoubleSide }));
  const flame = kit.trackMaterial(new THREE.MeshBasicMaterial({ color: look.flame }));
  const stem = kit.trackMaterial(new THREE.MeshStandardMaterial({ color: map === 'frozen' ? '#bfe6ff' : '#4a3426', roughness: 0.8 }));
  const flameGeometry = kit.track(map === 'frozen' ? new THREE.OctahedronGeometry(0.22) : new THREE.ConeGeometry(0.13, 0.34, 6));
  const stemGeometry = kit.track(new THREE.CylinderGeometry(0.05, 0.07, 0.5, 5));
  const flames: THREE.Object3D[] = [];
  let lights = 0;
  const up = new THREE.Vector3(0, 1, 0);

  for (const tunnel of tunnels) {
    const ends = [positions.get(tunnel.mouth)!, positions.get(tunnel.exit)!];
    const inside = tunnel.inside.map((id) => positions.get(id)!);
    // The bore starts a little way past each mouth tile, so the tile stays in the open.
    const points = [ends[0].clone().lerp(inside[0], 0.4), ...inside, ends[1].clone().lerp(inside[inside.length - 1], 0.4)].map(
      (p) => p.clone().add(new THREE.Vector3(0, LIFT, 0)),
    );
    const curve = new THREE.CatmullRomCurve3(points);
    const geometry = kit.track(new THREE.TubeGeometry(curve, points.length * 6, RADIUS, 11, false));
    // Rough the walls up, but leave the floor flat enough to walk. The
    // tube's normals point out of it, so the floor's point down.
    const position = geometry.attributes.position as THREE.BufferAttribute;
    const normal = geometry.attributes.normal as THREE.BufferAttribute;
    for (let i = 0; i < position.count; i++) {
      const ny = normal.getY(i);
      if (ny < -0.7) continue;
      const x = position.getX(i);
      const z = position.getZ(i);
      const bump = (Math.sin(x * 2.1 + z * 1.3) + Math.sin(z * 2.7 - x * 0.9)) * 0.14;
      position.setXYZ(i, x - normal.getX(i) * bump, position.getY(i) - ny * bump, z - normal.getZ(i) * bump);
    }
    geometry.computeVertexNormals();
    const bore = new THREE.Mesh(geometry, rock);
    bore.receiveShadow = true;
    kit.group.add(bore);

    // Each mouth: a ring of rock round the opening and darkness inside it.
    for (const t of [0, 1]) {
      const at = curve.getPointAt(t);
      const along = curve.getTangentAt(t);
      const facing = new THREE.Quaternion().setFromUnitVectors(new THREE.Vector3(0, 0, 1), along);
      const ring = new THREE.Mesh(kit.track(new THREE.TorusGeometry(RADIUS + 0.2, 0.5, 6, 14)), rock);
      ring.position.copy(at);
      ring.quaternion.copy(facing);
      ring.castShadow = true;
      kit.group.add(ring);
      const hole = new THREE.Mesh(kit.track(new THREE.CircleGeometry(RADIUS, 18)), dark);
      hole.position.copy(curve.getPointAt(t === 0 ? 0.06 : 0.94));
      hole.quaternion.copy(facing);
      kit.group.add(hole);
    }

    // A light on alternate walls every few spaces; only some cast light, to keep the cost down.
    const step = 3;
    for (let i = 1; i < inside.length; i += step) {
      const t = (i + 1) / (points.length - 1);
      const at = curve.getPointAt(Math.min(1, t));
      const along = curve.getTangentAt(Math.min(1, t));
      const side = new THREE.Vector3().crossVectors(along, up).normalize().multiplyScalar(((i / step) % 2 ? 1 : -1) * (RADIUS - 0.35));
      const base = at.clone().add(side).add(new THREE.Vector3(0, -0.2, 0));
      const torch = new THREE.Mesh(stemGeometry, stem);
      torch.position.copy(base);
      kit.group.add(torch);
      const fire = new THREE.Mesh(flameGeometry, flame);
      fire.position.copy(base).add(new THREE.Vector3(0, 0.38, 0));
      kit.group.add(fire);
      flames.push(fire);
      if (lights < (kit.detail < 1 ? 3 : 7) && ((i - 1) / step) % 2 === 0) {
        const light = new THREE.PointLight(look.light, 9, 11, 1.6);
        light.position.copy(fire.position);
        kit.group.add(light);
        lights++;
      }
    }
  }
  kit.animate((time) => {
    flames.forEach((fire, i) => {
      const flicker = 1 + Math.sin(time * 11 + i * 1.7) * 0.12 + Math.sin(time * 23 + i) * 0.06;
      fire.scale.set(1, flicker, 1);
      if (map === 'frozen') fire.rotation.y = time * 0.6 + i;
    });
  });
}
