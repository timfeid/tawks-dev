import * as THREE from 'three';
import { buildCharacter, type CharacterRig } from '../characters';
import type { CharacterDef, Member, RaceSetup, RaceSnap } from '../protocol';

type Rider = { rig: CharacterRig; racer: THREE.Group; lane: THREE.Group; bounce: number };

/** A colorful tabletop race course for Rocker Rally. The server owns positions;
 * this class only makes the course and smooths the latest snapshot. */
export class RockerTrack {
  readonly group = new THREE.Group();
  private setup: RaceSetup | null = null;
  private built = false;
  private riders = new Map<number, Rider>();
  private geometries: THREE.BufferGeometry[] = [];
  private materials: THREE.Material[] = [];

  constructor(scene: THREE.Scene) {
    this.group.name = 'party-rocker-rally';
    this.group.visible = false;
    scene.add(this.group);
  }

  private geometry<T extends THREE.BufferGeometry>(value: T) { this.geometries.push(value); return value; }
  private material<T extends THREE.Material>(value: T) { this.materials.push(value); return value; }

  private build(setup: RaceSetup) {
    this.setup = setup;
    const length = setup.finish - setup.start + 8;
    const track = new THREE.Mesh(
      this.geometry(new THREE.BoxGeometry(setup.half * 2, 0.45, length)),
      this.material(new THREE.MeshStandardMaterial({ color: '#f4d690', roughness: 0.88 })),
    );
    track.position.set(0, -0.24, (setup.finish + setup.start) / 2);
    track.receiveShadow = true;
    this.group.add(track);
    const stripe = this.material(new THREE.MeshStandardMaterial({ color: '#ea6a4b', roughness: 0.65 }));
    for (const x of [-setup.half + 0.45, -setup.half / 3, setup.half / 3, setup.half - 0.45]) {
      const line = new THREE.Mesh(this.geometry(new THREE.BoxGeometry(0.18, 0.03, length - 1)), stripe);
      line.position.set(x, 0.01, (setup.finish + setup.start) / 2);
      this.group.add(line);
    }
    const checkered = this.material(new THREE.MeshStandardMaterial({ color: '#25233a', roughness: 0.8 }));
    for (let x = -setup.half + 0.8; x < setup.half; x += 1.6) {
      for (let z = -0.3; z <= 0.3; z += 0.3) {
        const tile = new THREE.Mesh(this.geometry(new THREE.BoxGeometry(0.8, 0.04, 0.3)), checkered);
        tile.position.set(x, 0.03, setup.finish + z);
        this.group.add(tile);
      }
    }
    const boost = this.material(new THREE.MeshStandardMaterial({ color: '#5ee0b8', emissive: '#1a8b72', emissiveIntensity: 0.45 }));
    setup.boosts.forEach(([x, z]) => {
      const pad = new THREE.Mesh(this.geometry(new THREE.BoxGeometry(3.8, 0.05, 1.5)), boost);
      pad.position.set(x, 0.04, z);
      this.group.add(pad);
    });
    const mud = this.material(new THREE.MeshStandardMaterial({ color: '#76523c', roughness: 1 }));
    setup.mud.forEach(([x, z, radius]) => {
      const patch = new THREE.Mesh(this.geometry(new THREE.CircleGeometry(radius, 20)), mud);
      patch.rotation.x = -Math.PI / 2;
      patch.position.set(x, 0.045, z);
      this.group.add(patch);
    });
    this.built = true;
  }

  private rider(def: CharacterDef) {
    const racer = new THREE.Group();
    const spring = new THREE.Mesh(this.geometry(new THREE.CylinderGeometry(0.1, 0.1, 0.52, 10)), this.material(new THREE.MeshStandardMaterial({ color: '#dce9e0', metalness: 0.55, roughness: 0.35 })));
    spring.position.y = 0.34;
    const base = new THREE.Mesh(this.geometry(new THREE.BoxGeometry(0.9, 0.18, 1.15)), this.material(new THREE.MeshStandardMaterial({ color: def.accent, roughness: 0.6 })));
    base.position.y = 0.09;
    const head = new THREE.Mesh(this.geometry(new THREE.SphereGeometry(0.3, 14, 10)), this.material(new THREE.MeshStandardMaterial({ color: def.body, roughness: 0.65 })));
    head.position.set(0, 0.6, -0.27);
    racer.add(base, spring, head);
    const rig = buildCharacter(def);
    // The course is deliberately wide, so use a bold, toy-like rider scale.
    // At full-course framing they need to read as characters, not pins.
    rig.group.scale.setScalar(0.92);
    rig.group.position.y = 0.64;
    rig.group.rotation.y = Math.PI;
    racer.add(rig.group);
    racer.scale.setScalar(1.28);
    this.group.add(racer);
    return { rig, racer, lane: new THREE.Group(), bounce: 0 };
  }

  sync(snap: RaceSnap | null, setup: RaceSetup | null, members: Member[], roster: Map<string, CharacterDef>) {
    if (setup && !this.built) this.build(setup);
    if (!snap || !this.built) return;
    for (const player of snap.players) {
      const member = members.find(m => m.seat === player.seat);
      const def = member && roster.get(member.character);
      if (!def) continue;
      let rider = this.riders.get(player.seat);
      if (!rider) { rider = this.rider(def); this.riders.set(player.seat, rider); rider.racer.position.set(player.x, 0, player.z); }
      rider.racer.position.x = THREE.MathUtils.damp(rider.racer.position.x, player.x, 12, 1 / 60);
      rider.racer.position.z = THREE.MathUtils.damp(rider.racer.position.z, player.z, 12, 1 / 60);
      rider.bounce = Math.max(rider.bounce, player.bounce);
      rider.racer.visible = !player.finished;
    }
  }

  update(dt: number) {
    for (const rider of this.riders.values()) {
      rider.bounce = Math.max(0, rider.bounce - dt * 3.5);
      rider.racer.position.y = Math.sin(performance.now() * 0.02) * rider.bounce * 0.16;
    }
  }

  set visible(value: boolean) { this.group.visible = value; }
  dispose() { for (const rider of this.riders.values()) rider.rig.dispose(); for (const geometry of this.geometries) geometry.dispose(); for (const material of this.materials) material.dispose(); this.group.removeFromParent(); }
}
