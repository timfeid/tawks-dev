import * as THREE from 'three';
import { buildCharacter, type CharacterRig } from '../characters';
import type { CharacterDef, Member } from '../protocol';

export type RisingPad = { x: number; y: number; z: number };
export type RisingSetup = { game: 'rising'; radius: number; pad: number; step: number; pads: RisingPad[] };
export type RisingPlayer = {
  seat: number;
  x: number;
  y: number;
  z: number;
  vx: number;
  vz: number;
  air: boolean;
  out: boolean;
};
export type PadState = 'solid' | 'crumbling' | 'gone';
export type RisingSnap = {
  game: 'rising';
  time: number;
  limit: number;
  water: number;
  players: RisingPlayer[];
  pads: PadState[];
};

type Climber = { seat: number; characterId: string; rig: CharacterRig; yaw: number };

/** The tank's floor sits on the sea, so the flood reads as the sea coming in. */
const BOTTOM = -3;
const PAD_DEPTH = 0.45;

export class RisingField {
  readonly group = new THREE.Group();
  private climbers = new Map<number, Climber>();
  private pads: THREE.Mesh[] = [];
  private flood: THREE.Mesh<THREE.CylinderGeometry, THREE.MeshStandardMaterial>;
  private surface: THREE.Mesh<THREE.CircleGeometry, THREE.MeshStandardMaterial>;
  private solid = new THREE.MeshStandardMaterial({ color: '#c9b48a', roughness: 0.9, flatShading: true });
  private crumbling = new THREE.MeshStandardMaterial({ color: '#d9774a', roughness: 0.9, flatShading: true });
  private built = false;
  private setup: RisingSetup | null = null;

  constructor(scene: THREE.Scene) {
    this.group.name = 'rising-field';
    scene.add(this.group);
    this.flood = new THREE.Mesh(
      new THREE.CylinderGeometry(1, 1, 1, 48, 1, true),
      new THREE.MeshStandardMaterial({
        color: '#2f7fbf',
        transparent: true,
        opacity: 0.55,
        roughness: 0.2,
        side: THREE.DoubleSide,
        depthWrite: false,
      }),
    );
    this.surface = new THREE.Mesh(
      new THREE.CircleGeometry(1, 48),
      new THREE.MeshStandardMaterial({ color: '#5fb4e8', transparent: true, opacity: 0.7, roughness: 0.15 }),
    );
    this.surface.rotation.x = -Math.PI / 2;
    this.group.add(this.flood, this.surface);
  }

  set visible(visible: boolean) {
    this.group.visible = visible;
  }

  private build(setup: RisingSetup) {
    this.setup = setup;
    const radius = setup.radius;
    const top = Math.max(...setup.pads.map((pad) => pad.y), 0) + setup.step * 2;

    const floor = new THREE.Mesh(
      new THREE.CylinderGeometry(radius, radius, 0.6, 48),
      new THREE.MeshStandardMaterial({ color: '#8d8477', roughness: 0.95, flatShading: true }),
    );
    floor.position.y = -0.3;
    floor.receiveShadow = true;

    const wall = new THREE.Mesh(
      new THREE.CylinderGeometry(radius + 0.15, radius + 0.15, top - BOTTOM, 48, 1, true),
      new THREE.MeshStandardMaterial({
        color: '#dff3ff',
        transparent: true,
        opacity: 0.12,
        side: THREE.DoubleSide,
        depthWrite: false,
      }),
    );
    wall.position.y = (top + BOTTOM) / 2;
    const rimGeometry = new THREE.TorusGeometry(radius + 0.15, 0.12, 8, 64);
    const rimMaterial = new THREE.MeshStandardMaterial({ color: '#f2efe6', roughness: 0.6 });
    for (const y of [BOTTOM, top]) {
      const rim = new THREE.Mesh(rimGeometry, rimMaterial);
      rim.rotation.x = Math.PI / 2;
      rim.position.y = y;
      this.group.add(rim);
    }
    this.group.add(floor, wall);

    const padGeometry = new THREE.CylinderGeometry(setup.pad, setup.pad * 0.7, PAD_DEPTH, 10);
    for (const spot of setup.pads) {
      const pad = new THREE.Mesh(padGeometry, this.solid);
      pad.position.set(spot.x, spot.y - PAD_DEPTH / 2, spot.z);
      pad.castShadow = true;
      pad.receiveShadow = true;
      this.group.add(pad);
      this.pads.push(pad);
    }

    this.flood.scale.set(radius, 1, radius);
    this.surface.scale.set(radius, radius, 1);
    this.built = true;
  }

  sync(snap: RisingSnap | null, setup: RisingSetup | null, members: Member[], roster: Map<string, CharacterDef>) {
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
        rig.group.scale.setScalar(0.8);
        this.group.add(rig.group);
        climber = { seat: player.seat, characterId: def.id, rig, yaw: 0 };
        this.climbers.set(player.seat, climber);
      }
      if (Math.hypot(player.vx, player.vz) > 0.3) climber.yaw = Math.atan2(player.vx, player.vz);
      climber.rig.group.position.set(player.x, player.y, player.z);
      climber.rig.group.rotation.y = climber.yaw;
      climber.rig.group.visible = !player.out || player.y > snap.water - 1.5;
    }
  }

  update(snap: RisingSnap | null, time: number) {
    const water = snap?.water ?? BOTTOM;
    const depth = Math.max(0.01, water - BOTTOM);
    this.flood.scale.y = depth;
    this.flood.position.y = BOTTOM + depth / 2;
    this.surface.position.y = water + Math.sin(time * 1.7) * 0.04;
    this.surface.visible = water > BOTTOM;

    this.pads.forEach((pad, index) => {
      const state = snap?.pads[index] ?? 'solid';
      const spot = this.setup?.pads[index];
      pad.visible = state !== 'gone';
      pad.material = state === 'crumbling' ? this.crumbling : this.solid;
      if (!spot) return;
      const shiver = state === 'crumbling' ? 0.06 : 0;
      pad.position.set(
        spot.x + Math.sin(time * 41 + index) * shiver,
        spot.y - PAD_DEPTH / 2,
        spot.z + Math.cos(time * 37 + index) * shiver,
      );
    });

    for (const climber of this.climbers.values()) {
      const player = snap?.players.find((one) => one.seat === climber.seat);
      const speed = player ? Math.hypot(player.vx, player.vz) : 0;
      const pose = player?.out ? 'sad' : player?.air ? 'fall' : speed > 0.3 ? 'ride' : 'idle';
      climber.rig.animate(pose, time + climber.seat * 0.4, speed);
    }
  }

  dispose() {
    for (const climber of this.climbers.values()) climber.rig.dispose();
    this.climbers.clear();
    this.group.traverse((object) => {
      if (object instanceof THREE.Mesh) object.geometry.dispose();
    });
    this.solid.dispose();
    this.crumbling.dispose();
    this.group.removeFromParent();
  }
}
