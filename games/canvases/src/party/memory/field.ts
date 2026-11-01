import * as THREE from 'three';
import { buildCharacter, type CharacterRig } from '../characters';
import type { CharacterDef, Member } from '../protocol';

export type MemorySetup = { game: 'memory'; side: number; spacing: number; tile: number; half: number };
export type MemoryPhase = 'show' | 'recall' | 'break';
export type MemoryPlayer = {
  seat: number;
  x: number;
  z: number;
  progress: number;
  done: boolean;
  out: boolean;
  cleared: number;
};
export type MemorySnap = {
  game: 'memory';
  time: number;
  limit?: number;
  phase: MemoryPhase;
  round: number;
  length: number;
  lit: number | null;
  players: MemoryPlayer[];
};
export type MemoryEvent =
  | { kind: 'light'; tile: number; step: number }
  | { kind: 'go'; round: number }
  | { kind: 'stomp'; seat: number; tile: number; step: number }
  | { kind: 'wrong'; seat: number; tile: number; wanted: number }
  | { kind: 'done'; seat: number }
  | { kind: 'timeout'; seat: number }
  | { kind: 'survived'; round: number; left: number };

/** One colour per tile, so a sequence can be remembered by colour as well as place. */
const COLOURS = ['#ef4444', '#f97316', '#eab308', '#22c55e', '#14b8a6', '#3b82f6', '#8b5cf6', '#ec4899', '#f5f5f4'];
const WRONG = new THREE.Color('#dc2626');
const THICK = 0.3;

type Tile = { mesh: THREE.Mesh<THREE.BoxGeometry, THREE.MeshStandardMaterial>; base: THREE.Color; flash: number; flashColour: THREE.Color };
type Walker = { seat: number; characterId: string; rig: CharacterRig; yaw: number; speed: number };

export class MemoryField {
  readonly group = new THREE.Group();
  private tiles: Tile[] = [];
  private walkers = new Map<number, Walker>();
  private built = false;

  constructor(scene: THREE.Scene) {
    this.group.name = 'memory-field';
    scene.add(this.group);
  }

  set visible(visible: boolean) {
    this.group.visible = visible;
  }

  private build(setup: MemorySetup) {
    const span = setup.half * 2 + 1;
    const stone = new THREE.Mesh(
      new THREE.BoxGeometry(span, 6, span),
      new THREE.MeshStandardMaterial({ color: '#57534e', roughness: 0.9, flatShading: true }),
    );
    stone.position.y = -3;
    stone.receiveShadow = true;
    this.group.add(stone);

    const size = setup.tile * 2;
    for (let tile = 0; tile < setup.side * setup.side; tile++) {
      const base = new THREE.Color(COLOURS[tile % COLOURS.length]);
      const mesh = new THREE.Mesh(
        new THREE.BoxGeometry(size, THICK, size),
        new THREE.MeshStandardMaterial({ color: base.clone().multiplyScalar(0.45), roughness: 0.6, emissive: base, emissiveIntensity: 0 }),
      );
      const middle = (setup.side - 1) / 2;
      mesh.position.set(((tile % setup.side) - middle) * setup.spacing, THICK / 2, (Math.floor(tile / setup.side) - middle) * setup.spacing);
      mesh.receiveShadow = true;
      this.group.add(mesh);
      this.tiles.push({ mesh, base, flash: 0, flashColour: base.clone() });
    }
    this.built = true;
  }

  sync(snap: MemorySnap | null, setup: MemorySetup | null, members: Member[], roster: Map<string, CharacterDef>) {
    if (setup && !this.built) this.build(setup);
    if (!snap) return;

    for (const [seat, walker] of this.walkers) {
      if (snap.players.some((player) => player.seat === seat)) continue;
      walker.rig.dispose();
      this.walkers.delete(seat);
    }
    for (const player of snap.players) {
      const member = members.find((one) => one.seat === player.seat);
      const def = roster.get(member?.character ?? '');
      if (!def) continue;
      let walker = this.walkers.get(player.seat);
      if (!walker || walker.characterId !== def.id) {
        walker?.rig.dispose();
        const rig = buildCharacter(def);
        rig.group.scale.setScalar(0.9);
        this.group.add(rig.group);
        walker = { seat: player.seat, characterId: def.id, rig, yaw: 0, speed: 0 };
        this.walkers.set(player.seat, walker);
      }
      const next = new THREE.Vector3(player.x, THICK, player.z);
      const moved = next.clone().sub(walker.rig.group.position);
      walker.speed = moved.length() * 60;
      if (moved.lengthSq() > 0.0004) walker.yaw = Math.atan2(moved.x, moved.z);
      walker.rig.group.position.copy(next);
      walker.rig.group.rotation.y = walker.yaw;
    }
  }

  /** A tile flashes when stomped: its own colour when right, red when wrong. */
  stomp(tile: number, right: boolean) {
    const one = this.tiles[tile];
    if (!one) return;
    one.flash = 1;
    one.flashColour.copy(right ? one.base : WRONG);
  }

  update(snap: MemorySnap | null, time: number, dt: number) {
    this.tiles.forEach((tile, index) => {
      tile.flash = Math.max(0, tile.flash - dt * 2.5);
      const lit = snap?.phase === 'show' && snap.lit === index;
      const glow = lit ? 1 : tile.flash;
      tile.mesh.material.emissive.copy(lit ? tile.base : tile.flashColour);
      tile.mesh.material.emissiveIntensity = glow * 1.4;
      tile.mesh.position.y = THICK / 2 + glow * 0.15;
    });
    for (const walker of this.walkers.values()) {
      const player = snap?.players.find((one) => one.seat === walker.seat);
      const pose = player?.out ? 'sad' : player?.done ? 'cheer' : walker.speed > 0.5 ? 'ride' : 'idle';
      walker.rig.animate(pose, time + walker.seat * 0.4, player?.out ? 0 : walker.speed);
    }
  }

  dispose() {
    for (const walker of this.walkers.values()) walker.rig.dispose();
    this.walkers.clear();
    for (const tile of this.tiles) {
      tile.mesh.geometry.dispose();
      tile.mesh.material.dispose();
    }
    this.group.removeFromParent();
  }
}
