import * as THREE from 'three';
import type { BoutHost, Rider, Seat } from '@tawks/bout-sdk';
import { place } from '../board/models';
import { buildCar, buildRails, carMaterials } from '../../tracklines/rolling-stock';

/**
 * Minecart Rush's line through a quarry, laid with Tracklines' own rails and
 * cars at a size people can ride in.
 */

type Point = [number, number];
export type MinecartSetup = {
  game: 'minecart';
  straights: Point[][];
  branches: [Point[], Point[]][];
  fast: number[];
  gauge: number;
  /** How many carts the siding behind the gate holds, more on a bigger field. */
  siding?: number;
};
export type MinecartPlayer = {
  seat: number;
  x: number;
  z: number;
  heading: number;
  speed: number;
  lean: number;
  progress: number;
  waiting: boolean;
  stunned: boolean;
  finished: boolean;
  ride: number;
  wrong: number;
  crashes: number;
};
export type MinecartSnap = { game: 'minecart'; time: number; limit: number; players: MinecartPlayer[] };

/** Tracklines builds its kit for a map where the rails are 0.13 apart. */
const MAP_GAUGE = 0.13;
const GATE_SPACING = 3.2;
/** The siding's length in carts when the server has not said; it is built for four. */
const SIDING = 5;

type Cart = { seat: number; characterId: string; group: THREE.Group; rider: Rider; color: string };

const DIRT = new THREE.Color('#a48a6a');
const STONE = new THREE.Color('#7d7468');

const curve = (points: Point[]) => new THREE.CatmullRomCurve3(points.map(([x, z]) => new THREE.Vector3(x, 0, z)));

/** Frees a cart. The rider's parts belong to the host, so it frees them itself. */
function disposeCart(cart: Cart) {
  cart.rider.group.removeFromParent();
  cart.rider.dispose();
  cart.group.traverse((object) => {
    const mesh = object as THREE.Mesh;
    mesh.geometry?.dispose();
    const material = mesh.material;
    if (Array.isArray(material)) material.forEach((one) => one.dispose());
    else material?.dispose();
  });
  cart.group.removeFromParent();
}

export class MinecartField {
  readonly group = new THREE.Group();
  shake = 0;
  private carts = new Map<number, Cart>();
  private built = false;
  private scale = 0.9 / MAP_GAUGE;
  private gate = new THREE.Vector3();
  private finish = new THREE.Vector3();

  constructor(private host: BoutHost) {
    this.group.name = 'minecart-field';
    host.scene.add(this.group);
  }

  set visible(visible: boolean) {
    this.group.visible = visible;
  }

  /** Where the line starts and ends, for the camera before anyone rolls. */
  get start() {
    return this.gate;
  }

  private build(setup: MinecartSetup) {
    this.scale = setup.gauge / MAP_GAUGE;
    const first = setup.straights[0];
    const last = setup.straights[setup.straights.length - 1];
    this.gate.set(first[0][0], 0, first[0][1]);
    this.finish.set(last[last.length - 1][0], 0, last[last.length - 1][1]);

    const siding = GATE_SPACING * (setup.siding ?? SIDING);
    // The quarry floor reaches past the end of the siding however long it is.
    const north = this.gate.z + Math.max(24, siding + 8);
    const south = this.finish.z - 18;
    const ground = new THREE.PlaneGeometry(64, north - south, 24, 72);
    ground.rotateX(-Math.PI / 2);
    ground.translate(0, -0.02, (north + south) / 2);
    const position = ground.attributes.position as THREE.BufferAttribute;
    const colors = new Float32Array(position.count * 3);
    const colour = new THREE.Color();
    for (let i = 0; i < position.count; i++) {
      const x = position.getX(i);
      const z = position.getZ(i);
      // The quarry walls rise away from the line on both sides.
      const wall = Math.max(0, Math.abs(x) - 15);
      position.setY(i, -0.02 + wall * wall * 0.05 + Math.sin(z * 0.3 + x) * 0.15 * Math.min(1, wall));
      colour.copy(DIRT).lerp(STONE, Math.min(1, wall / 10) * 0.8 + Math.abs(Math.sin(x * 1.7 + z * 0.9)) * 0.12);
      colour.toArray(colors, i * 3);
    }
    ground.setAttribute('color', new THREE.BufferAttribute(colors, 3));
    ground.computeVertexNormals();
    const floor = new THREE.Mesh(ground, new THREE.MeshStandardMaterial({ vertexColors: true, roughness: 1, flatShading: true }));
    floor.receiveShadow = true;
    this.group.add(floor);

    const lay = (points: Point[], sleeper?: string) => {
      const path = curve(points);
      const rails = buildRails(path, path.getLength(), this.scale, sleeper);
      rails.traverse((object) => {
        object.castShadow = true;
        object.receiveShadow = true;
      });
      this.group.add(rails);
    };
    // A siding behind the gate for the carts still waiting their turn.
    lay([[this.gate.x, this.gate.z + siding], [this.gate.x, this.gate.z]]);
    for (const straight of setup.straights) lay(straight);
    setup.branches.forEach((pair, junction) => {
      pair.forEach((points, side) => {
        const slow = side !== setup.fast[junction];
        lay(points, slow ? '#4a3d30' : undefined);
        this.flag(points, slow);
        if (slow) this.rubble(points, junction * 2 + side);
      });
    });

    this.post(this.gate, '#f4f0e6');
    this.post(this.finish, '#ffd23f');
    this.dress(north, south);
    this.built = true;
  }

  /** A flag where each branch leaves the junction: green for the short way. */
  private flag(points: Point[], slow: boolean) {
    const [x, z] = points[Math.min(3, points.length - 1)];
    const side = Math.sign(x) || 1;
    const pole = new THREE.Mesh(new THREE.CylinderGeometry(0.08, 0.08, 2.6, 6), new THREE.MeshStandardMaterial({ color: '#5a4a3a' }));
    pole.position.set(x + side * 1.6, 1.3, z);
    const cloth = new THREE.Mesh(
      new THREE.BoxGeometry(0.9, 0.55, 0.05),
      new THREE.MeshStandardMaterial({ color: slow ? '#d94f3d' : '#4cc463', roughness: 0.8 }),
    );
    cloth.position.set(x + side * 2.05, 2.25, z);
    pole.castShadow = cloth.castShadow = true;
    this.group.add(pole, cloth);
  }

  /** Loose rock along the long branch, which is why it is slow. */
  private rubble(points: Point[], salt: number) {
    for (let i = 4; i < points.length - 3; i += 5) {
      const [x, z] = points[i];
      const out = (Math.sign(x) || 1) * (1.7 + ((i * 7 + salt) % 3) * 0.3);
      void place(this.group, 'rock_largeB', { x: x + out, z, scale: 0.7, yaw: i + salt });
    }
  }

  private post(at: THREE.Vector3, color: string) {
    const material = new THREE.MeshStandardMaterial({ color, roughness: 0.6 });
    for (const side of [-1, 1]) {
      const leg = new THREE.Mesh(new THREE.BoxGeometry(0.3, 4.2, 0.3), material);
      leg.position.set(at.x + side * 2.4, 2.1, at.z);
      leg.castShadow = true;
      this.group.add(leg);
    }
    const beam = new THREE.Mesh(new THREE.BoxGeometry(5.1, 0.45, 0.35), material);
    beam.position.set(at.x, 4.2, at.z);
    beam.castShadow = true;
    this.group.add(beam);
  }

  private dress(north: number, south: number) {
    let seed = 20261027;
    const random = () => {
      seed = (seed * 1664525 + 1013904223) % 4294967296;
      return seed / 4294967296;
    };
    const count = this.host.mobile ? 50 : 110;
    for (let i = 0; i < count; i++) {
      const side = random() < 0.5 ? -1 : 1;
      const x = side * (16 + random() * 14);
      const z = south + random() * (north - south);
      const wall = Math.abs(x) - 15;
      const roll = random();
      const name =
        roll < 0.4
          ? ['tree_pineTallA', 'tree_pineTallB', 'tree_pineRoundA'][Math.floor(random() * 3)]
          : ['rock_largeB', 'rock_tallC', 'rock_largeD'][Math.floor(random() * 3)];
      void place(this.group, name, { x, z, y: wall * wall * 0.05, scale: 1.3 + random() * 0.9, yaw: random() * 6 });
    }
  }

  private makeCart(seat: Seat): Cart | null {
    const character = this.host.character(seat.character);
    if (!character) return null;
    const group = new THREE.Group();
    const { car } = buildCar(carMaterials(character.body), { cabs: [1], open: true });
    car.scale.setScalar(this.scale);
    car.traverse((object) => {
      object.castShadow = true;
    });
    group.add(car);
    const rider = this.host.rider(character);
    rider.group.scale.setScalar(0.85);
    rider.group.position.y = 0.2 * this.scale;
    rider.group.rotation.order = 'YXZ';
    group.add(rider.group);
    this.group.add(group);
    return { seat: seat.seat, characterId: character.id, group, rider, color: character.body };
  }

  sync(snap: MinecartSnap | null, setup: MinecartSetup | null, seats: Seat[]) {
    if (setup && !this.built) this.build(setup);
    if (!snap) return;
    for (const [seat, cart] of this.carts) {
      if (snap.players.some((player) => player.seat === seat)) continue;
      disposeCart(cart);
      this.carts.delete(seat);
    }
    let queued = 0;
    for (const player of [...snap.players].sort((a, b) => a.seat - b.seat)) {
      const seat = seats.find((one) => one.seat === player.seat);
      if (!seat) continue;
      let cart = this.carts.get(player.seat);
      if (!cart || cart.characterId !== seat.character) {
        if (cart) disposeCart(cart);
        cart = this.makeCart(seat) ?? undefined;
        if (!cart) continue;
        this.carts.set(player.seat, cart);
      }
      const behind = player.waiting ? GATE_SPACING * ++queued : 0;
      cart.group.position.set(player.x, 0, player.z + behind);
      // Tracklines' cars run along x; the rider faces +z.
      const heading = player.waiting ? Math.PI : player.heading;
      cart.group.children[0].rotation.y = Math.atan2(-Math.cos(heading), Math.sin(heading));
      cart.rider.group.rotation.y = heading;
      cart.rider.group.rotation.z = -player.lean * 0.3;
    }
  }

  /** The cart the camera should follow: the viewer's, else whoever leads. */
  focus(snap: MinecartSnap | null, mySeat: number | null): MinecartPlayer | null {
    const players = snap?.players ?? [];
    const mine = players.find((player) => player.seat === mySeat && !player.waiting && !player.finished);
    if (mine) return mine;
    const rolling = players.filter((player) => !player.waiting && !player.finished);
    return rolling.sort((a, b) => b.progress - a.progress)[0] ?? null;
  }

  update(snap: MinecartSnap | null, time: number, dt: number) {
    this.shake = Math.max(0, this.shake - dt * 1.6);
    for (const cart of this.carts.values()) {
      const player = snap?.players.find((one) => one.seat === cart.seat);
      const pose = player?.finished ? 'cheer' : player?.stunned ? 'fall' : player?.waiting ? 'idle' : 'ride';
      cart.rider.animate(pose, time + cart.seat * 0.4, player?.speed ?? 0);
    }
  }

  dispose() {
    for (const cart of this.carts.values()) disposeCart(cart);
    this.carts.clear();
    this.group.removeFromParent();
  }
}
