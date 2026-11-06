/** An original low-poly hatchback, two draw calls: the painted body and its lamps. */

import * as THREE from 'three';
import { mergeGeometries } from 'three/examples/jsm/utils/BufferGeometryUtils.js';

function tinted(geometry: THREE.BufferGeometry, rgb: [number, number, number]) {
  const g = geometry.index ? geometry.toNonIndexed() : geometry;
  const count = g.getAttribute('position').count;
  const colors = new Float32Array(count * 3);
  for (let i = 0; i < count; i++) colors.set(rgb, i * 3);
  g.setAttribute('color', new THREE.BufferAttribute(colors, 3));
  g.deleteAttribute('uv');
  return g;
}

function box(w: number, h: number, d: number, x: number, y: number, z: number, rgb: [number, number, number]) {
  return tinted(new THREE.BoxGeometry(w, h, d).translate(x, y, z), rgb);
}

/** The cabin narrows toward the roof, like a glasshouse. */
function cabin(rgb: [number, number, number]) {
  const g = new THREE.BoxGeometry(1.72, 0.62, 2.3, 1, 1, 1);
  const pos = g.getAttribute('position') as THREE.BufferAttribute;
  for (let i = 0; i < pos.count; i++) {
    if (pos.getY(i) > 0) {
      pos.setX(i, pos.getX(i) * 0.84);
      pos.setZ(i, pos.getZ(i) * (pos.getZ(i) < 0 ? 0.7 : 0.86));
    }
  }
  g.computeVertexNormals();
  return tinted(g.translate(0, 1.27, 0.25), rgb);
}

function wheel(x: number, z: number) {
  const g = new THREE.CylinderGeometry(0.38, 0.38, 0.3, 10).rotateZ(Math.PI / 2).translate(x, 0.38, z);
  return tinted(g, [0.06, 0.06, 0.07]);
}

let shared: { body: THREE.BufferGeometry; lamps: THREE.BufferGeometry } | null = null;

function geometries() {
  if (shared) return shared;
  const white: [number, number, number] = [1, 1, 1];
  const glass: [number, number, number] = [0.09, 0.11, 0.14];
  const trim: [number, number, number] = [0.16, 0.16, 0.18];
  const body = mergeGeometries([
    box(1.92, 0.62, 4.2, 0, 0.66, 0, white),
    box(1.96, 0.18, 4.26, 0, 0.36, 0, trim),
    cabin(glass),
    box(1.5, 0.06, 1.7, 0, 1.6, 0.35, white),
    wheel(-0.92, -1.35),
    wheel(0.92, -1.35),
    wheel(-0.92, 1.35),
    wheel(0.92, 1.35),
  ])!;
  const lamps = mergeGeometries([
    box(0.42, 0.16, 0.06, -0.64, 0.78, -2.11, [1, 0.96, 0.8]),
    box(0.42, 0.16, 0.06, 0.64, 0.78, -2.11, [1, 0.96, 0.8]),
    box(0.46, 0.14, 0.06, -0.66, 0.8, 2.11, [0.9, 0.08, 0.06]),
    box(0.46, 0.14, 0.06, 0.66, 0.8, 2.11, [0.9, 0.08, 0.06]),
  ])!;
  shared = { body, lamps };
  return shared;
}

export type CarModel = { group: THREE.Group; lamps: THREE.MeshBasicMaterial };

/** A car facing -z, so heading h rotates it by -h about y. */
export function makeCar(color: string): CarModel {
  const { body, lamps } = geometries();
  const group = new THREE.Group();
  const paint = new THREE.MeshLambertMaterial({ color, vertexColors: true });
  group.add(new THREE.Mesh(body, paint));
  const lampMat = new THREE.MeshBasicMaterial({ vertexColors: true, color: 0x8a8a8a });
  group.add(new THREE.Mesh(lamps, lampMat));
  return { group, lamps: lampMat };
}

export function disposeCar(car: CarModel) {
  car.group.traverse((o) => {
    if (o instanceof THREE.Mesh) (o.material as THREE.Material).dispose();
  });
}
