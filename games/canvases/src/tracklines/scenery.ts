import * as THREE from 'three';
import geography from './north-america.json';
import { createMapTexture, MAP_HEIGHT, MAP_WIDTH, project } from './geography';

type Point = [number, number];

function clipRing(ring: Point[]): Point[] {
  let result = ring;
  for (const [axis, edge, sign] of [[0, -MAP_WIDTH / 2, 1], [0, MAP_WIDTH / 2, -1], [1, -MAP_HEIGHT / 2, 1], [1, MAP_HEIGHT / 2, -1]]) {
    const input = result;
    result = [];
    for (let i = 0; i < input.length; i++) {
      const a = input[i], b = input[(i + 1) % input.length];
      const aInside = (a[axis] - edge) * sign >= 0;
      const bInside = (b[axis] - edge) * sign >= 0;
      if (aInside) result.push(a);
      if (aInside !== bInside) {
        const t = (edge - a[axis]) / (b[axis] - a[axis]);
        result.push([a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]);
      }
    }
  }
  return result;
}

function inside(point: Point, ring: Point[]) {
  let result = false;
  for (let i = 0, j = ring.length - 1; i < ring.length; j = i++) {
    const a = ring[i], b = ring[j];
    if ((a[1] > point[1]) !== (b[1] > point[1]) && point[0] < (b[0] - a[0]) * (point[1] - a[1]) / (b[1] - a[1]) + a[0]) result = !result;
  }
  return result;
}

const projected = (ring: number[][]): Point[] => ring.map(([lon, lat]) => {
  const point = project(lon, lat);
  return [point.x, -point.z];
});

/** Stylized physical relief. Coastlines use geographic data; terrain is decorative. */
export function createScenery(cityPositions: THREE.Vector3[], paths: THREE.CatmullRomCurve3[]) {
  const group = new THREE.Group();
  group.name = 'landscape';
  const texture = createMapTexture();
  const mapMaterial = new THREE.MeshStandardMaterial({ map: texture, roughness: 0.96 });
  const coastMaterial = new THREE.MeshStandardMaterial({ color: '#68563a', roughness: 0.9 });
  const water = new THREE.Mesh(
    new THREE.PlaneGeometry(MAP_WIDTH, MAP_HEIGHT),
    new THREE.MeshStandardMaterial({ map: texture, roughness: 0.32, metalness: 0.18 }),
  );
  water.rotation.x = -Math.PI / 2;
  water.position.y = -0.36;
  water.receiveShadow = true;
  group.add(water);

  const landRings: Point[][] = [];
  const lakeRings = geography.lakes.map((polygon) => projected(polygon[0]));
  for (const country of geography.land) {
    for (const polygon of country.polygons) {
      const boundary = clipRing(projected(polygon[0]));
      if (boundary.length < 3) continue;
      landRings.push(boundary);
      const shape = new THREE.Shape(boundary.map(([x, y]) => new THREE.Vector2(x, y)));
      for (const ring of polygon.slice(1)) {
        const hole = clipRing(projected(ring));
        if (hole.length >= 3) shape.holes.push(new THREE.Path(hole.map(([x, y]) => new THREE.Vector2(x, y))));
      }
      const geometry = new THREE.ExtrudeGeometry(shape, { depth: 0.325, bevelEnabled: false, steps: 1, curveSegments: 1 });
      const positions = geometry.getAttribute('position');
      const uv = geometry.getAttribute('uv');
      for (let i = 0; i < positions.count; i++) uv.setXY(i, positions.getX(i) / MAP_WIDTH + 0.5, positions.getY(i) / MAP_HEIGHT + 0.5);
      const land = new THREE.Mesh(geometry, [mapMaterial, coastMaterial]);
      land.rotation.x = -Math.PI / 2;
      land.position.y = -0.36;
      land.castShadow = true;
      land.receiveShadow = true;
      group.add(land);
    }
  }
  const box = (width: number, height: number, depth: number, y: number, color: string, metalness = 0) => {
    const mesh = new THREE.Mesh(new THREE.BoxGeometry(width, height, depth), new THREE.MeshStandardMaterial({ color, roughness: 0.55, metalness }));
    mesh.position.y = y;
    mesh.castShadow = true;
    mesh.receiveShadow = true;
    group.add(mesh);
  };
  box(MAP_WIDTH + 0.4, 0.55, MAP_HEIGHT + 0.4, -0.67, '#273b39');
  box(MAP_WIDTH + 0.5, 0.045, MAP_HEIGHT + 0.5, -0.395, '#b99456', 0.55);
  box(MAP_WIDTH + 0.65, 0.12, MAP_HEIGHT + 0.65, -1, '#13262b');

  const routePoints = paths.flatMap((path) => path.getSpacedPoints(60));
  const clear = (x: number, z: number, radius: number) => {
    const point: Point = [x, -z];
    return Math.abs(x) < MAP_WIDTH / 2 - radius && Math.abs(z) < MAP_HEIGHT / 2 - radius
      && landRings.some((ring) => inside(point, ring)) && !lakeRings.some((ring) => inside(point, ring))
      && !cityPositions.some((city) => Math.hypot(city.x - x, city.z - z) < radius + 0.48)
      && !routePoints.some((p) => Math.hypot(p.x - x, p.z - z) < radius + 0.13);
  };
  let seed = 6281;
  const random = () => { seed = (Math.imul(seed, 1664525) + 1013904223) >>> 0; return seed / 4294967296; };
  const rock = new THREE.ConeGeometry(1, 1, 5);
  const stoneMaterials = ['#829083', '#6c8077', '#a4ac98'].map((color) => new THREE.MeshStandardMaterial({ color, roughness: 1, flatShading: true }));
  const snow = new THREE.MeshStandardMaterial({ color: '#e6eddd', roughness: 0.9, flatShading: true });
  // Rocky Mountains, Cascades, Sierra Nevada, and the Appalachian range.
  for (const [lon, lat, spreadX, spreadZ, count] of [[-111, 42, 1.1, 3.3, 85], [-121, 43, 0.45, 1.5, 30], [-118, 37, 0.55, 1.1, 24], [-82, 36.5, 0.7, 1.3, 32]]) {
    const center = project(lon, lat);
    for (let i = 0; i < count; i++) {
      const x = center.x + (random() - 0.5) * spreadX * 2;
      const z = center.z + (random() - 0.5) * spreadZ * 2;
      const radius = 0.14 + random() * 0.25;
      const height = radius * (1.4 + random() * 1.3);
      if (!clear(x, z, radius * 0.75)) continue;
      const peak = new THREE.Mesh(rock, stoneMaterials[i % stoneMaterials.length]);
      peak.scale.set(radius, height, radius * 0.75);
      peak.position.set(x, height / 2 - 0.03, z);
      peak.rotation.y = random() * Math.PI;
      peak.castShadow = true;
      peak.receiveShadow = true;
      group.add(peak);
      if (height > 0.58) {
        const cap = new THREE.Mesh(rock, snow);
        cap.scale.copy(peak.scale).multiplyScalar(0.32);
        cap.position.set(x, height * 0.84 - 0.03, z);
        cap.rotation.copy(peak.rotation);
        cap.castShadow = true;
        group.add(cap);
      }
    }
  }

  const trees: THREE.Vector3[] = [];
  for (const [lon, lat, dx, dz] of [[-120, 46, 1.1, 0.9], [-85, 35, 1.2, 1.4], [-88, 45, 1.2, 0.8], [-110, 41, 1.4, 2.8]]) {
    const center = project(lon, lat);
    for (let i = 0; i < 95; i++) {
      const x = center.x + (random() - 0.5) * dx * 2;
      const z = center.z + (random() - 0.5) * dz * 2;
      if (clear(x, z, 0.065)) trees.push(new THREE.Vector3(x, 0, z));
    }
  }
  const foliage = new THREE.InstancedMesh(new THREE.ConeGeometry(0.09, 0.29, 5), new THREE.MeshStandardMaterial({ color: '#284f40', roughness: 1 }), trees.length);
  const trunks = new THREE.InstancedMesh(new THREE.CylinderGeometry(0.019, 0.025, 0.12, 5), new THREE.MeshStandardMaterial({ color: '#736045', roughness: 1 }), trees.length);
  const transform = new THREE.Object3D();
  trees.forEach((position, i) => {
    const scale = 0.75 + random() * 0.85;
    transform.scale.setScalar(scale);
    transform.position.copy(position).setY(0.17 * scale);
    transform.updateMatrix();
    foliage.setMatrixAt(i, transform.matrix);
    transform.position.y = 0.02;
    transform.updateMatrix();
    trunks.setMatrixAt(i, transform.matrix);
    foliage.setColorAt(i, new THREE.Color().setHSL(0.4, 0.22, 0.19 + random() * 0.1));
  });
  foliage.castShadow = true;
  foliage.receiveShadow = true;
  group.add(foliage, trunks);
  return group;
}

export function createStation() {
  const group = new THREE.Group();
  const add = (geometry: THREE.BufferGeometry, color: string, x: number, y: number, z: number) => {
    const mesh = new THREE.Mesh(geometry, new THREE.MeshStandardMaterial({ color, roughness: 0.75 }));
    mesh.position.set(x, y, z);
    mesh.castShadow = true;
    mesh.receiveShadow = true;
    group.add(mesh);
    return mesh;
  };
  add(new THREE.BoxGeometry(0.42, 0.06, 0.27), '#c1b899', 0, 0.045, 0);
  add(new THREE.BoxGeometry(0.28, 0.2, 0.17), '#e4d3ac', 0, 0.16, 0);
  const roof = add(new THREE.CylinderGeometry(0.16, 0.16, 0.34, 3), '#3b575b', 0, 0.29, 0);
  roof.rotation.z = Math.PI / 2;
  roof.rotation.x = Math.PI / 2;
  add(new THREE.BoxGeometry(0.052, 0.1, 0.012), '#dfa85c', 0, 0.14, 0.091);
  for (const x of [-0.09, 0.09]) add(new THREE.BoxGeometry(0.04, 0.055, 0.012), '#40565b', x, 0.185, 0.091);
  return group;
}
