import * as THREE from 'three';
import { mergeGeometries } from 'three/examples/jsm/utils/BufferGeometryUtils.js';

/**
 * The rails and cars Tracklines draws on its map, built at map scale (a car
 * is about 0.4 long). Other games pass a `scale` to lay the same kit at
 * their own size.
 */

export function buildRails(path: THREE.Curve<THREE.Vector3>, length: number, scale = 1, sleeper = '#665744'): THREE.Group {
  const group = new THREE.Group();
  const metal = new THREE.MeshStandardMaterial({ color: '#c3c7bb', metalness: 0.65, roughness: 0.35 });
  for (const offset of [-0.065 * scale, 0.065 * scale]) {
    const points = Array.from({ length: 81 }, (_, i) => {
      const position = path.getPointAt(i / 80);
      const direction = path.getTangentAt(i / 80);
      return position.add(new THREE.Vector3(-direction.z * offset, 0.075 * scale, direction.x * offset));
    });
    group.add(new THREE.Mesh(new THREE.TubeGeometry(new THREE.CatmullRomCurve3(points), 80, 0.017 * scale, 5, false), metal));
  }
  const count = Math.max(3, Math.floor(length / (0.13 * scale)));
  const ties = new THREE.InstancedMesh(
    new THREE.BoxGeometry(0.045 * scale, 0.035 * scale, 0.23 * scale),
    new THREE.MeshStandardMaterial({ color: sleeper, roughness: 1 }),
    count,
  );
  const transform = new THREE.Object3D();
  for (let i = 0; i < count; i++) {
    const t = (i + 0.5) / count;
    transform.position.copy(path.getPointAt(t)).setY(0.032 * scale);
    const direction = path.getTangentAt(t);
    transform.rotation.y = Math.atan2(-direction.z, direction.x);
    transform.updateMatrix();
    ties.setMatrixAt(i, transform.matrix);
  }
  group.add(ties);
  return group;
}

export type CarMaterials = {
  body: THREE.MeshStandardMaterial;
  roof: THREE.MeshStandardMaterial;
  dark: THREE.MeshStandardMaterial;
  glass: THREE.MeshStandardMaterial;
  lamp: THREE.MeshBasicMaterial;
};

export function carMaterials(color: string): CarMaterials {
  return {
    body: new THREE.MeshStandardMaterial({ color, roughness: 0.45, metalness: 0.15 }),
    roof: new THREE.MeshStandardMaterial({ color: new THREE.Color(color).lerp(new THREE.Color('#ffffff'), 0.12), roughness: 0.5 }),
    dark: new THREE.MeshStandardMaterial({ color: '#253035', roughness: 0.7 }),
    glass: new THREE.MeshStandardMaterial({ color: '#b9e4f1', roughness: 0.22, metalness: 0.35 }),
    lamp: new THREE.MeshBasicMaterial({ color: '#fff3b0' }),
  };
}

export type CarOptions = {
  /** Which ends get a cab and a lamp: 1 is the +x end, -1 the -x end. */
  cabs?: number[];
  /** Leave off the roof and windows, for a cart someone rides in. */
  open?: boolean;
};

/** One car, running along x with its wheels on the rails at y = 0. */
export function buildCar(materials: CarMaterials, options: CarOptions = {}) {
  const { body, roof, dark, glass, lamp } = materials;
  const car = new THREE.Group();
  const lights: THREE.Mesh[] = [];
  const addBox = (dimensions: number[], position: number[], material: THREE.Material) => {
    const mesh = new THREE.Mesh(new THREE.BoxGeometry(...(dimensions as [number, number, number])), material);
    mesh.position.set(...(position as [number, number, number]));
    car.add(mesh);
    return mesh;
  };
  addBox([0.38, 0.07, 0.18], [0, 0.12, 0], dark);
  addBox([0.35, 0.14, 0.17], [0, 0.2, 0], body);
  if (!options.open) addBox([0.37, 0.035, 0.19], [0, 0.29, 0], roof);
  // Dark wheels, colored roof and body, and pale windows read clearly from above.
  for (const x of [-0.11, 0.11]) {
    for (const z of [-0.1, 0.1]) {
      const wheel = new THREE.Mesh(new THREE.CylinderGeometry(0.045, 0.045, 0.024, 10), dark);
      wheel.rotation.x = Math.PI / 2;
      wheel.position.set(x, 0.085, z);
      car.add(wheel);
    }
  }
  if (!options.open) {
    for (const x of [-0.1, 0, 0.1]) {
      for (const z of [-0.088, 0.088]) addBox([0.06, 0.052, 0.008], [x, 0.232, z], glass);
    }
  }
  for (const end of options.cabs ?? []) {
    if (!options.open) addBox([0.03, 0.065, 0.125], [end * 0.177, 0.225, 0], glass);
    const light = addBox([0.012, 0.028, 0.1], [end * 0.2, 0.165, 0], lamp);
    light.userData.direction = end;
    lights.push(light);
  }
  // Batch the repeated parts into four meshes per car instead of one draw per wheel/window.
  for (const material of [body, roof, dark, glass]) {
    const parts = car.children.filter((object) => object instanceof THREE.Mesh && object.material === material) as THREE.Mesh[];
    if (!parts.length) continue;
    const geometries = parts.map((part) => {
      part.updateMatrix();
      part.geometry.applyMatrix4(part.matrix);
      return part.geometry;
    });
    const merged = mergeGeometries(geometries);
    if (merged) {
      parts.forEach((part) => car.remove(part));
      geometries.forEach((geometry) => geometry.dispose());
      car.add(new THREE.Mesh(merged, material));
    }
  }
  return { car, lights };
}
