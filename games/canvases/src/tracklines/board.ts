import * as THREE from 'three';
import { mergeGeometries } from 'three/examples/jsm/utils/BufferGeometryUtils.js';
import { project } from './geography';
import { createScenery, createStation } from './scenery';

export type MapCity = { id: string; name: string; longitude: number; latitude: number };
export type MapRoute = { id: number; from: string; to: string; length: number; color: string; twin: number | null; owner: string | null };
type Track = {
  path: THREE.CatmullRomCurve3;
  length: number;
  slots: THREE.Mesh<THREE.BoxGeometry, THREE.MeshStandardMaterial>[];
  line: THREE.Mesh<THREE.TubeGeometry, THREE.MeshBasicMaterial>;
  cost: THREE.Sprite;
  rails: THREE.Group;
  highlight: THREE.Mesh<THREE.TubeGeometry, THREE.MeshBasicMaterial>;
  glow: 'none' | 'claimable' | 'focus';
};
type TrailStep = { route: number; forward: boolean };
type Train = {
  key: string;
  path: THREE.CatmullRomCurve3;
  length: number;
  group: THREE.Group;
  cars: THREE.Group[];
  lights: THREE.Mesh[];
  owner: string;
  color: string;
  spacing: number;
  halfLength: number;
  offset: number;
};

// A few long links bend inland or around the Great Lakes and Gulf coast.
// These are game routes, not representations of individual rail operators.
const WAYPOINTS: Record<number, [number, number][]> = {
  1: [[-113, 47.3], [-103, 47.2]],
  2: [[-90.4, 42.5], [-84.7, 41.1], [-76.3, 42.5]],
  17: [[-84.2, 41.1], [-78.7, 41.6]],
  56: [[-91.8, 30.8], [-85.3, 30.9], [-82.3, 29.9]],
  57: [[-119.1, 46.2], [-116.2, 43.6]],
  58: [[-73.75, 42.65], [-77.6, 41.6]],
  59: [[-120.2, 36], [-116.7, 34]],
  60: [[-78.6, 35.8], [-81.1, 32.1], [-81.7, 30.3]],
};

const LABEL_OFFSETS: Record<string, [number, number]> = {
  frostmere: [-0.45, -0.4], westwick: [-0.72, 0.16], saltmarsh: [-0.72, 0.1],
  dunmere: [-0.8, -0.28], elmshore: [0.2, 0.46], pinecrest: [0.2, 0.42],
  northgate: [0.05, -0.45], harrowdale: [-0.55, -0.3], thornfield: [0.65, 0.16],
  glassmoor: [0.38, -0.4], ironvale: [0.7, 0.16], kestrel_bay: [0.6, -0.35],
  ravenmoor: [-0.38, -0.4], cinderport: [0.65, 0.3], quarry_hill: [0.7, 0.35],
  juniper_cross: [0.28, 0.4], yarrow: [-0.48, 0.4],
};

// Dispose each shared resource once. SceneManager only disposes the renderer.
function disposeObject(root: THREE.Object3D) {
  const geometries = new Set<THREE.BufferGeometry>();
  const materials = new Set<THREE.Material>();
  const textures = new Set<THREE.Texture>();
  root.traverse((object) => {
    const mesh = object as THREE.Mesh;
    if (mesh.geometry) geometries.add(mesh.geometry);
    if (mesh.material) {
      for (const material of Array.isArray(mesh.material) ? mesh.material : [mesh.material]) {
        materials.add(material);
        const map = (material as THREE.MeshBasicMaterial).map;
        if (map) textures.add(map);
      }
    }
  });
  geometries.forEach((geometry) => geometry.dispose());
  materials.forEach((material) => material.dispose());
  textures.forEach((texture) => texture.dispose());
  root.removeFromParent();
}

export class TracklinesBoard {
  readonly group = new THREE.Group();
  readonly pickObjects: THREE.Object3D[] = [];
  private tracks = new Map<number, Track>();
  private trains = new Map<string, Train>();
  private ownershipKey = '';
  private point = new THREE.Vector3();
  private tangent = new THREE.Vector3();

  constructor(cities: MapCity[], routes: MapRoute[], colors: Record<string, string>) {
    this.group.name = 'tracklines-map';
    const positions = new Map(cities.map((city) => [city.id, project(city.longitude, city.latitude)]));
    const haloGeometry = new THREE.TorusGeometry(0.17, 0.035, 6, 24);
    const haloMaterial = new THREE.MeshStandardMaterial({ color: '#34443d', roughness: 0.8 });
    for (const city of cities) {
      const position = positions.get(city.id)!;
      const station = createStation();
      station.position.copy(position).add(new THREE.Vector3(0, 0, -0.2));
      this.group.add(station);
      const halo = new THREE.Mesh(haloGeometry, haloMaterial);
      halo.rotation.x = -Math.PI / 2;
      halo.position.copy(position).setY(0.08);
      this.group.add(halo);
      const label = this.makeLabel(city.name);
      const [dx, dz] = LABEL_OFFSETS[city.id] ?? [0, -0.4];
      label.position.copy(position).add(new THREE.Vector3(dx, 0.55, dz));
      this.group.add(label);
    }

    for (const route of routes) {
      const start = positions.get(route.from);
      const end = positions.get(route.to);
      if (!start || !end) continue;
      const middle = WAYPOINTS[route.id]?.map(([lon, lat]) => project(lon, lat)) ?? [];
      if (middle.length > 1 && middle[0].distanceTo(start) > middle[middle.length - 1].distanceTo(start)) middle.reverse();
      if (!middle.length) {
        const direction = end.clone().sub(start);
        const side = new THREE.Vector3(-direction.z, 0, direction.x).normalize();
        const bend = route.twin !== null ? (route.id < route.twin ? 0.25 : -0.25) : ((route.id % 3) - 1) * 0.1;
        middle.push(start.clone().lerp(end, 0.5).addScaledVector(side, bend));
      }
      const fullPath = new THREE.CatmullRomCurve3([start, ...middle, end]);
      const trim = Math.min(0.16 / fullPath.getLength(), 0.15);
      const path = new THREE.CatmullRomCurve3(Array.from({ length: 49 }, (_, i) => fullPath.getPointAt(trim + (1 - 2 * trim) * i / 48)));
      const length = path.getLength();
      const rails = this.makeRails(path, length);
      const line = new THREE.Mesh(
        new THREE.TubeGeometry(path, 72, 0.045, 5, false),
        new THREE.MeshBasicMaterial({ color: colors[route.color], transparent: true, opacity: 0.62 }),
      );
      line.position.y = 0.025;
      const cost = this.makeCostBadge(route.length, colors[route.color]);
      cost.position.copy(path.getPointAt(0.5)).setY(0.24);
      // A flattened tube under the line reads as an outline around it.
      const highlight = new THREE.Mesh(
        new THREE.TubeGeometry(path, 72, 0.14, 8, false),
        new THREE.MeshBasicMaterial({ color: '#fff7a8', transparent: true, opacity: 0, depthWrite: false }),
      );
      highlight.scale.y = 0.12;
      highlight.renderOrder = -1;
      highlight.visible = false;
      const track: Track = { path, length, slots: [], line, cost, rails, highlight, glow: 'none' };
      this.group.add(line);
      this.group.add(cost);
      track.rails.visible = false;
      this.group.add(track.rails);
      this.group.add(track.highlight);

      const guide = new THREE.Mesh(
        new THREE.TubeGeometry(path, 72, 0.026, 4, false),
        new THREE.MeshBasicMaterial({ color: '#4e5348', transparent: true, opacity: 0.22 }),
      );
      guide.position.y = 0.012;
      this.group.add(guide);
      const hitArea = new THREE.Mesh(
        new THREE.TubeGeometry(path, 72, 0.14, 4, false),
        new THREE.MeshBasicMaterial({ visible: false }),
      );
      hitArea.userData.routeId = route.id;
      this.pickObjects.push(hitArea);
      this.group.add(hitArea);
      this.tracks.set(route.id, track);
    }
    this.group.add(createScenery([...positions.values()], [...this.tracks.values()].map((track) => track.path)));
  }

  private makeLabel(text: string): THREE.Sprite {
    const canvas = document.createElement('canvas');
    const ctx = canvas.getContext('2d')!;
    ctx.font = '600 27px system-ui, sans-serif';
    canvas.width = Math.ceil(ctx.measureText(text).width) + 24;
    canvas.height = 46;
    ctx.font = '600 27px system-ui, sans-serif';
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    ctx.lineWidth = 6;
    ctx.lineJoin = 'round';
    ctx.strokeStyle = '#172e2ee0';
    ctx.strokeText(text, canvas.width / 2, 23);
    ctx.fillStyle = '#f5edda';
    ctx.fillText(text, canvas.width / 2, 23);
    const texture = new THREE.CanvasTexture(canvas);
    texture.colorSpace = THREE.SRGBColorSpace;
    const sprite = new THREE.Sprite(new THREE.SpriteMaterial({ map: texture, depthTest: false }));
    sprite.scale.set(canvas.width / 135, canvas.height / 135, 1);
    sprite.renderOrder = 30;
    return sprite;
  }

  private makeCostBadge(length: number, color: string): THREE.Sprite {
    const canvas = document.createElement('canvas');
    canvas.width = 72;
    canvas.height = 72;
    const ctx = canvas.getContext('2d')!;
    ctx.fillStyle = '#17231fe8';
    ctx.beginPath();
    ctx.arc(36, 36, 28, 0, Math.PI * 2);
    ctx.fill();
    ctx.lineWidth = 6;
    ctx.strokeStyle = color;
    ctx.stroke();
    ctx.beginPath();
    ctx.arc(36, 36, 32.5, 0, Math.PI * 2);
    ctx.lineWidth = 2;
    ctx.strokeStyle = '#f6f1df';
    ctx.stroke();
    ctx.fillStyle = '#fffdf2';
    ctx.font = '700 34px system-ui, sans-serif';
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    ctx.fillText(String(length), 36, 37);
    const texture = new THREE.CanvasTexture(canvas);
    texture.colorSpace = THREE.SRGBColorSpace;
    const sprite = new THREE.Sprite(new THREE.SpriteMaterial({ map: texture, transparent: true, depthTest: false }));
    sprite.scale.set(0.34, 0.34, 1);
    sprite.renderOrder = 25;
    return sprite;
  }

  private makeRails(path: THREE.CatmullRomCurve3, length: number): THREE.Group {
    const group = new THREE.Group();
    const metal = new THREE.MeshStandardMaterial({ color: '#c3c7bb', metalness: 0.65, roughness: 0.35 });
    for (const offset of [-0.065, 0.065]) {
      const points = Array.from({ length: 81 }, (_, i) => {
        const position = path.getPointAt(i / 80);
        const direction = path.getTangentAt(i / 80);
        return position.add(new THREE.Vector3(-direction.z * offset, 0.075, direction.x * offset));
      });
      group.add(new THREE.Mesh(new THREE.TubeGeometry(new THREE.CatmullRomCurve3(points), 80, 0.017, 5, false), metal));
    }
    const count = Math.max(3, Math.floor(length / 0.13));
    const ties = new THREE.InstancedMesh(
      new THREE.BoxGeometry(0.045, 0.035, 0.23),
      new THREE.MeshStandardMaterial({ color: '#665744', roughness: 1 }),
      count,
    );
    const transform = new THREE.Object3D();
    for (let i = 0; i < count; i++) {
      const t = (i + 0.5) / count;
      transform.position.copy(path.getPointAt(t)).setY(0.032);
      const direction = path.getTangentAt(t);
      transform.rotation.y = Math.atan2(-direction.z, direction.x);
      transform.updateMatrix();
      ties.setMatrixAt(i, transform.matrix);
    }
    group.add(ties);
    return group;
  }

  private makeTrain(key: string, path: THREE.CatmullRomCurve3, owner: string, color: string, offset: number): Train {
    const length = path.getLength();
    const group = new THREE.Group();
    group.name = `train-${key}`;
    const body = new THREE.MeshStandardMaterial({ color, roughness: 0.45, metalness: 0.15 });
    const roof = new THREE.MeshStandardMaterial({ color: new THREE.Color(color).lerp(new THREE.Color('#ffffff'), 0.12), roughness: 0.5 });
    const dark = new THREE.MeshStandardMaterial({ color: '#253035', roughness: 0.7 });
    const glass = new THREE.MeshStandardMaterial({ color: '#b9e4f1', roughness: 0.22, metalness: 0.35 });
    const lamp = new THREE.MeshBasicMaterial({ color: '#fff3b0' });
    const cars: THREE.Group[] = [];
    const lights: THREE.Mesh[] = [];
    const count = length > 3 ? 4 : length > 1.65 ? 3 : length > 1 ? 2 : 1;
    const scale = Math.min(1, length / 0.9);
    const addBox = (parent: THREE.Group, dimensions: number[], position: number[], material: THREE.Material) => {
      const mesh = new THREE.Mesh(new THREE.BoxGeometry(...dimensions as [number, number, number]), material);
      mesh.position.set(...position as [number, number, number]);
      parent.add(mesh);
      return mesh;
    };
    for (let i = 0; i < count; i++) {
      const car = new THREE.Group();
      car.scale.setScalar(scale);
      addBox(car, [0.38, 0.07, 0.18], [0, 0.12, 0], dark);
      addBox(car, [0.35, 0.14, 0.17], [0, 0.2, 0], body);
      addBox(car, [0.37, 0.035, 0.19], [0, 0.29, 0], roof);
      // Dark wheels, colored roof and body, and pale windows read clearly from above.
      for (const x of [-0.11, 0.11]) {
        for (const z of [-0.1, 0.1]) {
          const wheel = new THREE.Mesh(new THREE.CylinderGeometry(0.045, 0.045, 0.024, 10), dark);
          wheel.rotation.x = Math.PI / 2;
          wheel.position.set(x, 0.085, z);
          car.add(wheel);
        }
      }
      for (const x of [-0.1, 0, 0.1]) {
        for (const z of [-0.088, 0.088]) addBox(car, [0.06, 0.052, 0.008], [x, 0.232, z], glass);
      }
      // Cabs at both ends let the train reverse at a station without teleporting cars.
      for (const end of [-1, 1]) {
        if ((end === 1 && i !== 0) || (end === -1 && i !== count - 1)) continue;
        addBox(car, [0.03, 0.065, 0.125], [end * 0.177, 0.225, 0], glass);
        const light = addBox(car, [0.012, 0.028, 0.1], [end * 0.2, 0.165, 0], lamp);
        light.userData.direction = end;
        lights.push(light);
      }
      // Batch the repeated parts into four meshes per car instead of one draw per wheel/window.
      for (const material of [body, roof, dark, glass]) {
        const parts = car.children.filter((object) => object instanceof THREE.Mesh && object.material === material) as THREE.Mesh[];
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
      group.add(car);
      cars.push(car);
    }
    this.group.add(group);
    return {
      key, path, length, group, cars, lights, owner, color,
      spacing: 0.43 * scale,
      halfLength: (0.4 + (count - 1) * 0.43) * scale / 2,
      offset,
    };
  }

  // Splits one player's routes into as few back-and-forth runs as possible:
  // repeatedly take the longest trail through the routes not yet covered.
  private trails(owned: MapRoute[]): TrailStep[][] {
    const remaining = new Set(owned.map((route) => route.id));
    const byCity = new Map<string, MapRoute[]>();
    for (const route of owned) {
      for (const city of [route.from, route.to]) byCity.set(city, [...(byCity.get(city) ?? []), route]);
    }
    const weight = (id: number) => this.tracks.get(id)?.length ?? 0;
    const result: TrailStep[][] = [];
    while (remaining.size > 0) {
      let best: TrailStep[] = [];
      let bestWeight = -1;
      let budget = 50_000;
      const walk = (city: string, trail: TrailStep[], total: number) => {
        if (total > bestWeight) {
          best = [...trail];
          bestWeight = total;
        }
        for (const route of byCity.get(city) ?? []) {
          if (!remaining.has(route.id) || budget-- <= 0) continue;
          remaining.delete(route.id);
          const forward = route.from === city;
          trail.push({ route: route.id, forward });
          walk(forward ? route.to : route.from, trail, total + weight(route.id));
          trail.pop();
          remaining.add(route.id);
        }
      };
      for (const city of [...byCity.keys()].sort()) walk(city, [], 0);
      if (!best.length) break;
      if (best[0].route > best[best.length - 1].route) {
        best = best.reverse().map((step) => ({ route: step.route, forward: !step.forward }));
      }
      best.forEach((step) => remaining.delete(step.route));
      result.push(best);
    }
    return result;
  }

  private trailPath(trail: TrailStep[]): THREE.CatmullRomCurve3 | null {
    const points: THREE.Vector3[] = [];
    for (const step of trail) {
      const track = this.tracks.get(step.route);
      if (!track) return null;
      const spaced = track.path.getSpacedPoints(Math.max(8, Math.ceil(track.length / 0.06)));
      if (!step.forward) spaced.reverse();
      points.push(...spaced);
    }
    return points.length > 1 ? new THREE.CatmullRomCurve3(points, false, 'centripetal') : null;
  }

  private syncTrains(routes: MapRoute[], playerColor: (id: string) => string) {
    const owned = routes.filter((route) => route.owner && this.tracks.has(route.id));
    const ownershipKey = owned.map((route) => `${route.id}:${route.owner}:${playerColor(route.owner!)}`).join(',');
    if (ownershipKey === this.ownershipKey) return;
    this.ownershipKey = ownershipKey;

    const wanted = new Map<string, { trail: TrailStep[]; owner: string; color: string }>();
    const owners = [...new Set(owned.map((route) => route.owner!))];
    for (const owner of owners) {
      const color = playerColor(owner);
      for (const trail of this.trails(owned.filter((route) => route.owner === owner))) {
        const key = `${owner}|${color}|${trail.map((step) => `${step.forward ? '' : '-'}${step.route}`).join('.')}`;
        wanted.set(key, { trail, owner, color });
      }
    }
    for (const [key, train] of this.trains) {
      if (wanted.has(key)) continue;
      disposeObject(train.group);
      this.trains.delete(key);
    }
    for (const [key, { trail, owner, color }] of wanted) {
      if (this.trains.has(key)) continue;
      const path = this.trailPath(trail);
      if (path) this.trains.set(key, this.makeTrain(key, path, owner, color, trail[0].route * 1.71));
    }
  }

  paint(
    routes: MapRoute[],
    colors: Record<string, string>,
    playerColor: (id: string) => string,
    claimable: Set<number>,
    selected: number | null,
    hovered: number | null,
    ticketHighlight: Set<number> = new Set(),
  ) {
    const choosing = claimable.size > 0;
    for (const route of routes) {
      const track = this.tracks.get(route.id);
      if (!track) continue;
      const color = route.owner ? playerColor(route.owner) : colors[route.color];
      const ticketPath = ticketHighlight.has(route.id);
      const canClaim = !route.owner && claimable.has(route.id);
      const pointed = canClaim && (hovered === route.id || selected === route.id);
      const focused = ticketPath || selected === route.id || pointed;
      const faded = choosing && !canClaim && !focused;
      track.line.material.color.set(ticketPath ? '#67e8f9' : color);
      track.line.material.opacity = focused || canClaim ? 1 : route.owner ? 0.42 : faded ? 0.3 : 0.62;
      track.line.position.y = canClaim || focused ? 0.04 : 0.025;
      const costScale = route.owner ? 0.3 : pointed ? 0.58 : canClaim || focused ? 0.5 : faded ? 0.32 : 0.4;
      track.cost.scale.set(costScale, costScale, 1);
      track.cost.material.opacity = route.owner ? 0.55 : faded ? 0.5 : 1;
      track.cost.renderOrder = canClaim || focused ? 27 : 25;
      track.glow = ticketPath || pointed ? 'focus' : canClaim ? 'claimable' : 'none';
      track.highlight.visible = track.glow !== 'none';
      track.highlight.material.color.set(ticketPath ? '#67e8f9' : pointed ? '#f59e0b' : '#16211d');
      track.rails.visible = !!route.owner;
    }
    this.syncTrains(routes, playerColor);
  }

  animate(seconds: number) {
    for (const track of this.tracks.values()) {
      if (track.glow === 'focus') {
        track.highlight.material.opacity = 0.85 + Math.sin(seconds * 5) * 0.1;
      } else if (track.glow === 'claimable') {
        track.highlight.material.opacity = 0.7 + Math.sin(seconds * 3) * 0.2;
      }
    }
    for (const train of this.trains.values()) {
      const margin = train.halfLength + 0.04;
      const distance = Math.max(0.02, train.length - margin * 2);
      const duration = Math.max(2.5, distance / 0.55);
      const dwell = 0.85;
      const cycle = duration * 2 + dwell * 2;
      const phase = (seconds + train.offset) % cycle;
      const outbound = phase < duration + dwell;
      const progress = outbound ? Math.min(phase / duration, 1) : 1 - Math.min((phase - duration - dwell) / duration, 1);
      const center = margin + distance * progress;
      for (let i = 0; i < train.cars.length; i++) {
        const position = center + ((train.cars.length - 1) / 2 - i) * train.spacing;
        const t = THREE.MathUtils.clamp(position / train.length, 0, 1);
        train.path.getPointAt(t, this.point);
        train.path.getTangentAt(t, this.tangent);
        train.cars[i].position.copy(this.point).setY(0.03);
        train.cars[i].rotation.y = Math.atan2(-this.tangent.z, this.tangent.x);
      }
      for (const light of train.lights) light.visible = light.userData.direction === (outbound ? 1 : -1);
    }
  }

  dispose() {
    disposeObject(this.group);
    this.tracks.clear();
    this.trains.clear();
    this.ownershipKey = '';
    this.pickObjects.length = 0;
  }
}
