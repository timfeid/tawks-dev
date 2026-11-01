import * as THREE from 'three';
import { buildCharacter, type CharacterRig } from '../characters';
import { place } from '../board/models';
import type { CharacterDef, Member } from '../protocol';

/**
 * The island's peak, standing on its own for King of the Hill: the same
 * flat-shaded ground and the same pines and boulders the board is dressed
 * with, so the bout looks like somewhere on the island rather than an arena.
 */

export type HillSetup = { game: 'hill'; radius: number; peak: number; summit: number };
export type HillPlayer = {
  seat: number;
  x: number;
  z: number;
  y: number;
  held: number;
  out: boolean;
  king: boolean;
};
export type HillSnap = { game: 'hill'; time: number; limit: number; players: HillPlayer[] };

type Climber = { seat: number; characterId: string; rig: CharacterRig; yaw: number };

const GRASS = new THREE.Color('#78c266');
const DEEP = new THREE.Color('#3f8f4c');
const ROCK = new THREE.Color('#8d8477');

export class HillField {
  readonly group = new THREE.Group();
  private climbers = new Map<number, Climber>();
  private ring: THREE.Mesh<THREE.RingGeometry, THREE.MeshBasicMaterial>;
  private built = false;
  private radius = 20;
  private peak = 7.5;

  constructor(scene: THREE.Scene) {
    this.group.name = 'hill-field';
    scene.add(this.group);
    this.ring = new THREE.Mesh(
      new THREE.RingGeometry(0, 1, 44),
      new THREE.MeshBasicMaterial({ color: '#ffd23f', transparent: true, opacity: 0.22, depthWrite: false }),
    );
    this.ring.rotation.x = -Math.PI / 2;
    this.group.add(this.ring);
  }

  set visible(visible: boolean) {
    this.group.visible = visible;
  }

  /** The same curve the server walks players over. */
  private ground(x: number, z: number) {
    const reach = Math.hypot(x, z) / this.radius;
    if (reach >= 1) return 0;
    const falloff = 1 - reach;
    return this.peak * falloff * falloff * (3 - 2 * falloff);
  }

  private build(setup: HillSetup) {
    this.radius = setup.radius;
    this.peak = setup.peak;
    const across = 64;
    const geometry = new THREE.PlaneGeometry(this.radius * 2.6, this.radius * 2.6, across, across);
    geometry.rotateX(-Math.PI / 2);
    const position = geometry.attributes.position as THREE.BufferAttribute;
    const colors = new Float32Array(position.count * 3);
    const colour = new THREE.Color();
    for (let i = 0; i < position.count; i++) {
      const x = position.getX(i);
      const z = position.getZ(i);
      const y = this.ground(x, z);
      position.setY(i, y);
      const high = Math.min(1, y / Math.max(0.1, this.peak));
      colour.copy(GRASS).lerp(DEEP, high);
      if (high > 0.7) colour.lerp(ROCK, (high - 0.7) / 0.3);
      colour.toArray(colors, i * 3);
    }
    geometry.setAttribute('color', new THREE.BufferAttribute(colors, 3));
    geometry.computeVertexNormals();
    const hill = new THREE.Mesh(
      geometry,
      new THREE.MeshStandardMaterial({ vertexColors: true, roughness: 0.96, flatShading: true }),
    );
    hill.receiveShadow = true;
    this.group.add(hill);

    // Dressed from the board's own kit, thinning out towards the top.
    let seed = 20260101;
    const random = () => {
      seed = (seed * 1664525 + 1013904223) % 4294967296;
      return seed / 4294967296;
    };
    for (let i = 0; i < 90; i++) {
      const angle = random() * Math.PI * 2;
      const out = Math.sqrt(0.18 + random() * 0.8) * this.radius;
      const x = Math.cos(angle) * out;
      const z = Math.sin(angle) * out;
      const y = this.ground(x, z);
      if (y > this.peak * 0.8) continue;
      const roll = random();
      const name =
        roll < 0.45
          ? ['tree_pineTallA', 'tree_pineTallB', 'tree_pineRoundA'][Math.floor(random() * 3)]
          : roll < 0.7
            ? ['rock_largeB', 'rock_tallC', 'rock_largeD'][Math.floor(random() * 3)]
            : 'plant_bushLarge';
      void place(this.group, name, { x, z, y, scale: 1.4 + random() * 0.6, yaw: random() * 6 });
    }

    this.ring.geometry.dispose();
    this.ring.geometry = new THREE.RingGeometry(setup.summit - 0.25, setup.summit, 44);
    this.ring.position.y = this.ground(0, 0) + 0.06;
    this.built = true;
  }

  sync(snap: HillSnap | null, setup: HillSetup | null, members: Member[], roster: Map<string, CharacterDef>) {
    if (setup && !this.built) this.build(setup);
    if (!snap) return;

    for (const [seat, climber] of this.climbers) {
      if (snap.players.some((player) => player.seat === seat)) continue;
      climber.rig.dispose();
      this.climbers.delete(seat);
    }
    for (const player of snap.players) {
      const member = members.find((one) => one.seat === player.seat);
      const def = roster.get(member?.character ?? '');
      if (!def) continue;
      let climber = this.climbers.get(player.seat);
      if (!climber || climber.characterId !== def.id) {
        climber?.rig.dispose();
        const rig = buildCharacter(def);
        rig.group.scale.setScalar(0.9);
        this.group.add(rig.group);
        climber = { seat: player.seat, characterId: def.id, rig, yaw: 0 };
        this.climbers.set(player.seat, climber);
      }
      const next = new THREE.Vector3(player.x, player.y + 0.1, player.z);
      const moved = next.clone().sub(climber.rig.group.position);
      if (moved.lengthSq() > 0.0004) climber.yaw = Math.atan2(moved.x, moved.z);
      climber.rig.group.position.copy(next);
      climber.rig.group.rotation.y = climber.yaw;
      climber.rig.group.visible = !player.out;
    }
  }

  update(snap: HillSnap | null, time: number, _dt: number) {
    const king = snap?.players.some((player) => player.king) ?? false;
    this.ring.material.opacity = king ? 0.45 + Math.sin(time * 6) * 0.15 : 0.22;
    for (const climber of this.climbers.values()) {
      const player = snap?.players.find((one) => one.seat === climber.seat);
      climber.rig.animate(player?.king ? 'cheer' : 'ride', time + climber.seat * 0.4, player?.out ? 0 : 5);
    }
  }

  dispose() {
    for (const climber of this.climbers.values()) climber.rig.dispose();
    this.climbers.clear();
    this.group.removeFromParent();
  }
}
