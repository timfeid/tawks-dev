/**
 * Original low-poly cars for Port Run: four parked classes, the crew's van
 * and the police cruiser with a light bar. Bodies carry vertex colours so a
 * single tint paints them; parked cars of a class share one instanced mesh.
 */

import * as THREE from 'three';
import { mergeGeometries } from 'three/examples/jsm/utils/BufferGeometryUtils.js';
import type { Class, Damage } from './protocol';

type RGB = [number, number, number];
const PAINT: RGB = [1, 1, 1];
const GLASS: RGB = [0.05, 0.06, 0.08];
const TRIM: RGB = [0.11, 0.11, 0.12];
const TYRE: RGB = [0.04, 0.04, 0.05];
const DOOR: RGB = [0.92, 0.92, 0.9];

function tinted(g: THREE.BufferGeometry, rgb: RGB) {
  const out = g.index ? g.toNonIndexed() : g;
  const n = out.getAttribute('position').count;
  const c = new Float32Array(n * 3);
  for (let i = 0; i < n; i++) c.set(rgb, i * 3);
  out.setAttribute('color', new THREE.BufferAttribute(c, 3));
  out.deleteAttribute('uv');
  return out;
}

function box(w: number, h: number, d: number, x: number, y: number, z: number, rgb: RGB) {
  return tinted(new THREE.BoxGeometry(w, h, d).translate(x, y, z), rgb);
}

function cabin(w: number, h: number, d: number, y: number, z: number, taper: number) {
  const g = new THREE.BoxGeometry(w, h, d);
  const pos = g.getAttribute('position') as THREE.BufferAttribute;
  for (let i = 0; i < pos.count; i++) {
    if (pos.getY(i) > 0) {
      pos.setX(i, pos.getX(i) * taper);
      pos.setZ(i, pos.getZ(i) * (pos.getZ(i) < 0 ? taper * 0.85 : taper));
    }
  }
  g.computeVertexNormals();
  return tinted(g.translate(0, y, z), GLASS);
}

function wheel(x: number, z: number, r: number) {
  return tinted(new THREE.CylinderGeometry(r, r, 0.3, 10).rotateZ(Math.PI / 2).translate(x, r, z), TYRE);
}

export type Shape = { width: number; length: number; height: number; wheel: number; wheelbase: number };

export const SHAPES: Record<Class, Shape> = {
  sedan: { width: 1.86, length: 4.6, height: 1.45, wheel: 0.36, wheelbase: 1.42 },
  suv: { width: 1.96, length: 4.8, height: 1.8, wheel: 0.42, wheelbase: 1.5 },
  hatch: { width: 1.76, length: 3.95, height: 1.48, wheel: 0.34, wheelbase: 1.25 },
  pickup: { width: 2.0, length: 5.4, height: 1.85, wheel: 0.43, wheelbase: 1.75 },
  van: { width: 2.0, length: 5.1, height: 2.2, wheel: 0.4, wheelbase: 1.65 },
  cruiser: { width: 1.9, length: 4.9, height: 1.5, wheel: 0.37, wheelbase: 1.5 },
};

const cache = new Map<Class, { body: THREE.BufferGeometry; front: [THREE.BufferGeometry, THREE.BufferGeometry]; rear: THREE.BufferGeometry }>();

/** The car's body (painted, glass, tyres) and its lamps, facing -z. */
export function geometries(kind: Class) {
  const hit = cache.get(kind);
  if (hit) return hit;
  const s = SHAPES[kind];
  const w = s.width;
  const l = s.length;
  const r = s.wheel;
  const wb = s.wheelbase;
  const parts: THREE.BufferGeometry[] = [];
  const sill = r + 0.05;
  switch (kind) {
    case 'sedan':
    case 'cruiser':
      parts.push(box(w, 0.62, l, 0, sill + 0.31, 0, PAINT), cabin(w * 0.9, 0.58, l * 0.48, sill + 0.9, l * 0.04, 0.84));
      if (kind === 'cruiser') {
        parts.push(box(w + 0.02, 0.5, l * 0.36, 0, sill + 0.3, 0.05, DOOR), box(w * 0.7, 0.18, 0.12, 0, sill + 0.2, -l / 2 - 0.08, TRIM));
      }
      break;
    case 'suv':
      parts.push(box(w, 0.78, l, 0, sill + 0.39, 0, PAINT), cabin(w * 0.94, 0.72, l * 0.62, sill + 1.12, l * 0.08, 0.92), box(w * 0.92, 0.06, l * 0.5, 0, sill + 1.5, l * 0.08, TRIM));
      break;
    case 'hatch':
      parts.push(box(w, 0.6, l, 0, sill + 0.3, 0, PAINT), cabin(w * 0.9, 0.6, l * 0.56, sill + 0.9, l * 0.12, 0.86));
      break;
    case 'pickup':
      parts.push(
        box(w, 0.72, l, 0, sill + 0.36, 0, PAINT),
        cabin(w * 0.94, 0.7, l * 0.3, sill + 1.07, -l * 0.08, 0.92),
        box(0.1, 0.45, l * 0.38, -w / 2 + 0.05, sill + 0.95, l * 0.28, PAINT),
        box(0.1, 0.45, l * 0.38, w / 2 - 0.05, sill + 0.95, l * 0.28, PAINT),
        box(w, 0.45, 0.1, 0, sill + 0.95, l / 2 - 0.05, PAINT),
      );
      break;
    case 'van':
      parts.push(box(w, 1.7, l * 0.78, 0, sill + 0.85, l * 0.11, PAINT), box(w, 0.9, l * 0.24, 0, sill + 0.45, -l * 0.38, PAINT), cabin(w * 0.96, 0.7, l * 0.2, sill + 1.25, -l * 0.33, 0.95));
      break;
  }
  parts.push(box(w + 0.04, 0.16, l + 0.04, 0, sill, 0, TRIM));
  for (const [x, z] of [
    [-w / 2 + 0.08, -wb],
    [w / 2 - 0.08, -wb],
    [-w / 2 + 0.08, wb],
    [w / 2 - 0.08, wb],
  ])
    parts.push(wheel(x, z, r));
  const lamp = (x: number, rgb: RGB, back: boolean) => box(0.36, 0.14, 0.05, x, sill + 0.42, back ? l / 2 + 0.02 : -l / 2 - 0.02, rgb);
  const front: [THREE.BufferGeometry, THREE.BufferGeometry] = [lamp(-w * 0.33, [1, 0.97, 0.85], false), lamp(w * 0.33, [1, 0.97, 0.85], false)];
  const rear = mergeGeometries([lamp(-w * 0.34, [0.9, 0.05, 0.04], true), lamp(w * 0.34, [0.9, 0.05, 0.04], true)])!;
  const out = { body: mergeGeometries(parts)!, front, rear };
  cache.set(kind, out);
  return out;
}

/** Pushes panels in by their damage; front is -z, left is -x. */
export function dent(base: THREE.BufferGeometry, kind: Class, damage: Damage): THREE.BufferGeometry {
  const g = base.clone();
  const s = SHAPES[kind];
  const pos = g.getAttribute('position') as THREE.BufferAttribute;
  const [front, rear, left, right] = damage.panels.map((p) => 1 - p);
  for (let i = 0; i < pos.count; i++) {
    const x = pos.getX(i);
    const y = pos.getY(i);
    const z = pos.getZ(i);
    const wobble = Math.sin(x * 7.1 + z * 3.3 + y * 5.7) * 0.5 + 0.5;
    let nx = x;
    let nz = z;
    let ny = y;
    const fz = (-z - s.length * 0.2) / (s.length * 0.3);
    if (fz > 0) {
      nz += front * 0.55 * Math.min(1, fz) * (0.6 + 0.4 * wobble);
      ny -= front * 0.12 * Math.min(1, fz) * wobble;
    }
    const rz = (z - s.length * 0.2) / (s.length * 0.3);
    if (rz > 0) nz -= rear * 0.5 * Math.min(1, rz) * (0.6 + 0.4 * wobble);
    if (x < 0) nx += left * 0.32 * Math.min(1, -x / (s.width / 2)) * (0.5 + 0.5 * wobble);
    if (x > 0) nx -= right * 0.32 * Math.min(1, x / (s.width / 2)) * (0.5 + 0.5 * wobble);
    pos.setXYZ(i, nx, ny, nz);
  }
  g.computeVertexNormals();
  return g;
}

export type VehicleModel = {
  group: THREE.Group;
  body: THREE.Mesh<THREE.BufferGeometry, THREE.MeshLambertMaterial>;
  heads: [THREE.Mesh<THREE.BufferGeometry, THREE.MeshBasicMaterial>, THREE.Mesh<THREE.BufferGeometry, THREE.MeshBasicMaterial>];
  tails: THREE.Mesh<THREE.BufferGeometry, THREE.MeshBasicMaterial>;
  bar: { red: THREE.MeshBasicMaterial; blue: THREE.MeshBasicMaterial } | null;
  kind: Class;
  dented: string;
};

export function makeVehicle(kind: Class, colour: string): VehicleModel {
  const g = geometries(kind);
  const group = new THREE.Group();
  const body = new THREE.Mesh(g.body, new THREE.MeshLambertMaterial({ color: colour, vertexColors: true }));
  group.add(body);
  const head = () => new THREE.Mesh(g.front[0], new THREE.MeshBasicMaterial({ vertexColors: true, color: 0x777777 }));
  const heads: VehicleModel['heads'] = [head(), new THREE.Mesh(g.front[1], new THREE.MeshBasicMaterial({ vertexColors: true, color: 0x777777 }))];
  const tails = new THREE.Mesh(g.rear, new THREE.MeshBasicMaterial({ vertexColors: true, color: 0x555555 }));
  group.add(heads[0], heads[1], tails);
  let bar: VehicleModel['bar'] = null;
  if (kind === 'cruiser') {
    const s = SHAPES.cruiser;
    const red = new THREE.MeshBasicMaterial({ color: 0x400000 });
    const blue = new THREE.MeshBasicMaterial({ color: 0x000a40 });
    const y = s.wheel + 0.05 + 1.22;
    const left = new THREE.Mesh(new THREE.BoxGeometry(0.62, 0.14, 0.3).translate(-0.34, y, 0.1), red);
    const right = new THREE.Mesh(new THREE.BoxGeometry(0.62, 0.14, 0.3).translate(0.34, y, 0.1), blue);
    group.add(left, right);
    bar = { red, blue };
  }
  return { group, body, heads, tails, bar, kind, dented: '' };
}

/** Re-dents a car when its damage changes. */
export function applyDamage(model: VehicleModel, damage: Damage) {
  const key = damage.panels.map((p) => p.toFixed(2)).join(',');
  model.heads[0].visible = damage.lights[0];
  model.heads[1].visible = damage.lights[1];
  if (key === model.dented) return;
  model.dented = key;
  const old = model.body.geometry;
  const clean = damage.panels.every((p) => p > 0.99);
  model.body.geometry = clean ? geometries(model.kind).body : dent(geometries(model.kind).body, model.kind, damage);
  if (old !== geometries(model.kind).body) old.dispose();
}

export function disposeVehicle(model: VehicleModel) {
  if (model.body.geometry !== geometries(model.kind).body) model.body.geometry.dispose();
  model.group.traverse((o) => {
    if (o instanceof THREE.Mesh) (o.material as THREE.Material).dispose();
  });
}

/** One instanced mesh per class for parked cars, each instance painted its own colour. */
export function parkedFleet(kind: Class, count: number) {
  const mesh = new THREE.InstancedMesh(geometries(kind).body, new THREE.MeshLambertMaterial({ vertexColors: true }), Math.max(1, count));
  mesh.count = count;
  mesh.instanceMatrix.setUsage(THREE.DynamicDrawUsage);
  return mesh;
}
