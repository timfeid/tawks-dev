import * as THREE from 'three';
import { buildCharacter, type CharacterRig } from '../characters';
import { headingOf, turnToward } from '../facing';
import type { CharacterDef, Member } from '../protocol';

/**
 * Bomb Relay: a straight lane with sliding gates across it and a pen at each
 * end. Only the carrier and the runner waiting in the far pen are on the
 * lane; everyone else stands on the benches along the sides.
 */

export type RelaySetup = {
  game: 'relay';
  rule: string;
  half_x: number;
  end_z: number;
  pen_z: number;
  pen_radius: number;
  gate_gap: number;
  gate_thick: number;
  gates: number[];
  bench_x: number;
};

export type RelayPlayer = {
  seat: number;
  x: number;
  z: number;
  vx: number;
  vz: number;
  out: boolean;
  holding: boolean;
  waiting: boolean;
  /** How many times they have handed the bomb on. */
  passes: number;
};

export type RelaySnap = {
  game: 'relay';
  time: number;
  limit: number;
  holder: number | null;
  receiver: number | null;
  fuse: number;
  lit: number;
  round: number;
  heading: number;
  /** Where each gate's gap is centred right now. */
  gates: number[];
  players: RelayPlayer[];
};

type Runner = { seat: number; characterId: string; rig: CharacterRig };
type Gate = { left: THREE.Mesh; right: THREE.Mesh; z: number };
type Puff = { sprite: THREE.Sprite; life: number };

const WALL_HEIGHT = 1.1;

function puffTexture() {
  const canvas = document.createElement('canvas');
  canvas.width = 64;
  canvas.height = 64;
  const ctx = canvas.getContext('2d')!;
  const gradient = ctx.createRadialGradient(32, 32, 3, 32, 32, 31);
  gradient.addColorStop(0, 'rgba(255,255,255,0.95)');
  gradient.addColorStop(1, 'rgba(255,255,255,0)');
  ctx.fillStyle = gradient;
  ctx.fillRect(0, 0, 64, 64);
  const texture = new THREE.CanvasTexture(canvas);
  texture.colorSpace = THREE.SRGBColorSpace;
  return texture;
}

export class RelayField {
  readonly group = new THREE.Group();
  private runners = new Map<number, Runner>();
  private gates: Gate[] = [];
  private pens: THREE.Mesh<THREE.RingGeometry, THREE.MeshBasicMaterial>[] = [];
  private bomb: THREE.Group;
  private spark: THREE.Mesh<THREE.SphereGeometry, THREE.MeshStandardMaterial>;
  private puffs: Puff[] = [];
  private puffMap = puffTexture();
  private geometries: THREE.BufferGeometry[] = [];
  private materials: THREE.Material[] = [];
  private setup: RelaySetup | null = null;
  shake = 0;

  constructor(scene: THREE.Scene) {
    this.group.name = 'relay-field';
    scene.add(this.group);
    this.bomb = new THREE.Group();
    const shell = new THREE.Mesh(
      this.track(new THREE.SphereGeometry(0.42, 16, 14)),
      this.material(new THREE.MeshStandardMaterial({ color: '#23252f', roughness: 0.45 })),
    );
    const fuse = new THREE.Mesh(
      this.track(new THREE.CylinderGeometry(0.05, 0.05, 0.35, 8)),
      this.material(new THREE.MeshStandardMaterial({ color: '#8a5a33', roughness: 0.9 })),
    );
    fuse.position.y = 0.5;
    this.spark = new THREE.Mesh(
      this.track(new THREE.SphereGeometry(0.13, 10, 8)),
      this.material(new THREE.MeshStandardMaterial({ color: '#ffbf47', emissive: '#ff7a1c', emissiveIntensity: 1.4 })),
    );
    this.spark.position.y = 0.7;
    this.bomb.add(shell, fuse, this.spark);
    this.bomb.visible = false;
    this.group.add(this.bomb);
  }

  set visible(visible: boolean) {
    this.group.visible = visible;
  }

  private track<T extends THREE.BufferGeometry>(geometry: T): T {
    this.geometries.push(geometry);
    return geometry;
  }

  private material<T extends THREE.Material>(material: T): T {
    this.materials.push(material);
    return material;
  }

  private build(setup: RelaySetup) {
    this.setup = setup;
    const { half_x: halfX, end_z: endZ } = setup;
    const lane = new THREE.Mesh(
      this.track(new THREE.BoxGeometry(halfX * 2, 0.6, endZ * 2)),
      this.material(new THREE.MeshStandardMaterial({ color: '#e9d6a8', roughness: 0.92 })),
    );
    lane.position.y = -0.3;
    lane.receiveShadow = true;
    this.group.add(lane);

    const rail = this.material(new THREE.MeshStandardMaterial({ color: '#e07a3f', roughness: 0.6 }));
    for (const side of [-1, 1]) {
      const edge = new THREE.Mesh(this.track(new THREE.BoxGeometry(0.3, 0.5, endZ * 2 + 0.6)), rail);
      edge.position.set(side * (halfX + 0.15), 0.25, 0);
      const end = new THREE.Mesh(this.track(new THREE.BoxGeometry(halfX * 2 + 0.6, 0.5, 0.3)), rail);
      end.position.set(0, 0.25, side * (endZ + 0.15));
      this.group.add(edge, end);
    }

    const bench = this.material(new THREE.MeshStandardMaterial({ color: '#8fbf7a', roughness: 0.95 }));
    for (const side of [-1, 1]) {
      const strip = new THREE.Mesh(this.track(new THREE.BoxGeometry(3, 0.4, endZ * 1.2)), bench);
      strip.position.set(side * setup.bench_x, -0.2, 0);
      strip.receiveShadow = true;
      this.group.add(strip);
    }

    for (const side of [-1, 1]) {
      const pen = new THREE.Mesh(
        this.track(new THREE.RingGeometry(setup.pen_radius + 0.4, setup.pen_radius + 0.75, 32)),
        this.material(new THREE.MeshBasicMaterial({ color: '#ffd23f', transparent: true, opacity: 0.35, depthWrite: false })),
      );
      pen.rotation.x = -Math.PI / 2;
      pen.position.set(0, 0.03, side * setup.pen_z);
      this.group.add(pen);
      this.pens.push(pen);
    }

    const wall = this.material(new THREE.MeshStandardMaterial({ color: '#3d4a63', roughness: 0.7 }));
    for (const z of setup.gates) {
      const left = new THREE.Mesh(this.track(new THREE.BoxGeometry(1, WALL_HEIGHT, setup.gate_thick)), wall);
      const right = new THREE.Mesh(this.track(new THREE.BoxGeometry(1, WALL_HEIGHT, setup.gate_thick)), wall);
      for (const part of [left, right]) {
        part.position.set(0, WALL_HEIGHT / 2, z);
        part.castShadow = true;
        this.group.add(part);
      }
      this.gates.push({ left, right, z });
    }
    this.placeGates(setup.gates.map(() => 0));
  }

  /** Each gate is two walls either side of its gap, stretched to the rails. */
  private placeGates(gaps: number[]) {
    if (!this.setup) return;
    const { half_x: halfX, gate_gap: gap } = this.setup;
    this.gates.forEach((gate, index) => {
      const centre = gaps[index] ?? 0;
      const leftEnd = centre - gap / 2;
      const rightEnd = centre + gap / 2;
      const leftWidth = Math.max(0.01, leftEnd + halfX);
      const rightWidth = Math.max(0.01, halfX - rightEnd);
      gate.left.scale.x = leftWidth;
      gate.left.position.x = -halfX + leftWidth / 2;
      gate.right.scale.x = rightWidth;
      gate.right.position.x = halfX - rightWidth / 2;
    });
  }

  sync(snap: RelaySnap | null, setup: RelaySetup | null, members: Member[], roster: Map<string, CharacterDef>) {
    if (setup && !this.setup) this.build(setup);
    if (!snap) return;

    for (const [seat, runner] of this.runners) {
      const member = members.find((one) => one.seat === seat);
      if (snap.players.some((player) => player.seat === seat) && member?.character === runner.characterId) continue;
      runner.rig.dispose();
      this.runners.delete(seat);
    }
    for (const player of snap.players) {
      if (this.runners.has(player.seat)) continue;
      const member = members.find((one) => one.seat === player.seat);
      const def = roster.get(member?.character ?? '');
      if (!def) continue;
      const rig = buildCharacter(def);
      rig.group.scale.setScalar(0.95);
      this.group.add(rig.group);
      this.runners.set(player.seat, { seat: player.seat, characterId: def.id, rig });
    }

    this.placeGates(snap.gates);
    const holder = snap.players.find((player) => player.seat === snap.holder);
    this.bomb.visible = !!holder;
    if (holder) this.bomb.position.set(holder.x, 2.35, holder.z);
  }

  effect(event: { kind: string; x?: number; z?: number }) {
    if (event.kind !== 'boom' || event.x === undefined || event.z === undefined) return;
    for (let i = 0; i < 18; i++) {
      const at = new THREE.Vector3(event.x, 0.8, event.z).add(
        new THREE.Vector3(Math.random() - 0.5, Math.random(), Math.random() - 0.5).multiplyScalar(4),
      );
      const sprite = new THREE.Sprite(
        new THREE.SpriteMaterial({ map: this.puffMap, color: i % 2 ? '#ff9f1c' : '#ff3b52', transparent: true, depthWrite: false }),
      );
      sprite.position.copy(at);
      sprite.scale.setScalar(0.8 + Math.random() * 0.7);
      this.group.add(sprite);
      this.puffs.push({ sprite, life: 0.8 });
    }
    this.shake = 0.5;
  }

  update(snap: RelaySnap | null, time: number, dt: number) {
    this.shake = Math.max(0, this.shake - dt * 1.6);
    const heading = snap?.heading ?? 1;
    this.pens.forEach((pen, index) => {
      const live = snap?.receiver !== null && snap?.receiver !== undefined && (index === 0 ? -1 : 1) === heading;
      pen.material.opacity = live ? 0.45 + Math.sin(time * 6) * 0.15 : 0.18;
    });

    for (const runner of this.runners.values()) {
      const player = snap?.players.find((one) => one.seat === runner.seat);
      if (!player) continue;
      runner.rig.group.position.set(player.x, 0, player.z);
      const speed = Math.hypot(player.vx, player.vz);
      const yaw = runner.rig.group.rotation.y;
      if (speed > 0.3) runner.rig.group.rotation.y = turnToward(yaw, headingOf(player.vx, player.vz), dt);
      else if (player.waiting || player.out) runner.rig.group.rotation.y = turnToward(yaw, player.x > 0 ? -Math.PI / 2 : Math.PI / 2, dt);
      runner.rig.group.scale.setScalar(player.holding ? 1.06 : 0.95);
      const pose = player.out ? 'fall' : speed > 0.8 ? 'ride' : 'idle';
      runner.rig.animate(pose, time + runner.seat * 0.3, speed > 0.8 ? 7 : 0);
    }

    if (this.bomb.visible) {
      const urgency = Math.max(0.6, 8 - (snap?.fuse ?? 5));
      this.spark.material.emissiveIntensity = 0.6 + Math.abs(Math.sin(time * urgency)) * 1.8;
      this.bomb.rotation.y = time * 1.6;
      this.bomb.position.y = 2.35 + Math.sin(time * 4) * 0.08;
    }

    for (let i = this.puffs.length - 1; i >= 0; i--) {
      const puff = this.puffs[i];
      puff.life -= dt;
      puff.sprite.position.y += dt * 1.6;
      puff.sprite.material.opacity = Math.max(0, puff.life);
      if (puff.life <= 0) {
        puff.sprite.material.dispose();
        puff.sprite.removeFromParent();
        this.puffs.splice(i, 1);
      }
    }
  }

  dispose() {
    for (const runner of this.runners.values()) runner.rig.dispose();
    this.runners.clear();
    for (const puff of this.puffs) puff.sprite.material.dispose();
    this.puffs = [];
    this.puffMap.dispose();
    this.geometries.forEach((geometry) => geometry.dispose());
    this.materials.forEach((material) => material.dispose());
    this.group.removeFromParent();
  }
}
