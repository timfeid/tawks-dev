import * as THREE from 'three';
import type { ArenaPlayer, ArenaSetup, ArenaSnap, GameEvent } from '../protocol';
import { ledgeSections, type LedgeSection } from './ledge-layout';

type LedgeSetup = Extract<ArenaSetup, { game: 'ledge' }>;

const STONE = ['#9aa3b5', '#8a93a6', '#7c859a'];
const CRACK = new THREE.Color('#f59e0b');
const WINDUP = '#fb7185';
const BRACE = '#93c5fd';
const SEAM = 0.035;
const SLAB = 0.9;

type Slab = {
  mesh: THREE.Mesh<THREE.ExtrudeGeometry, THREE.MeshStandardMaterial>;
  section: LedgeSection;
  home: THREE.Vector3;
  state: number;
  /** Seconds since it started falling. */
  dropping: number;
  spin: THREE.Vector3;
};

type Marker = {
  wedge: THREE.Mesh<THREE.CircleGeometry, THREE.MeshBasicMaterial>;
  brace: THREE.Mesh<THREE.RingGeometry, THREE.MeshBasicMaterial>;
  windupAt: number | null;
};

type Wave = { mesh: THREE.Mesh<THREE.RingGeometry, THREE.MeshBasicMaterial>; life: number; grow: number };

function sectorShape(section: LedgeSection) {
  const inner = section.inner + 0.05;
  const outer = section.outer - 0.05;
  const from = section.from + SEAM / 2;
  const to = section.to - SEAM / 2;
  const shape = new THREE.Shape();
  shape.moveTo(Math.cos(from) * inner, Math.sin(from) * inner);
  shape.absarc(0, 0, outer, from, to, false);
  shape.lineTo(Math.cos(to) * inner, Math.sin(to) * inner);
  shape.absarc(0, 0, inner, to, from, true);
  return shape;
}

/**
 * Ledge Shuffle's floor: stone sections round a central pillar that crack,
 * shake and drop on the server's schedule, plus each player's shove wedge
 * and brace ring so a wind-up can be seen coming.
 */
export class LedgeFloor {
  readonly group = new THREE.Group();
  private setup: LedgeSetup | null = null;
  private slabs: Slab[] = [];
  private core: THREE.Mesh<THREE.CylinderGeometry, THREE.MeshStandardMaterial> | null = null;
  private markers = new Map<number, Marker>();
  private waves: Wave[] = [];
  private wedgeGeometry: THREE.CircleGeometry | null = null;
  private braceGeometry = new THREE.RingGeometry(0.72, 0.9, 28);
  private waveGeometry = new THREE.RingGeometry(0.8, 1, 32);
  shake = 0;

  constructor(parent: THREE.Object3D) {
    this.group.name = 'ledge-floor';
    parent.add(this.group);
  }

  build(setup: LedgeSetup) {
    if (this.setup && JSON.stringify(this.setup.rings) === JSON.stringify(setup.rings)) return;
    this.clear();
    this.setup = setup;
    for (const section of ledgeSections(setup.rings)) {
      const geometry = new THREE.ExtrudeGeometry(sectorShape(section), { depth: SLAB, bevelEnabled: false, curveSegments: 6 });
      geometry.rotateX(Math.PI / 2);
      // Centred on itself, so a falling slab tumbles about its own middle.
      const mid = (section.from + section.to) / 2;
      const out = (section.inner + section.outer) / 2;
      const home = new THREE.Vector3(Math.cos(mid) * out, 0, Math.sin(mid) * out);
      geometry.translate(-home.x, 0, -home.z);
      const mesh = new THREE.Mesh(
        geometry,
        new THREE.MeshStandardMaterial({ color: STONE[section.ring % STONE.length], roughness: 0.92 }),
      );
      mesh.castShadow = true;
      mesh.receiveShadow = true;
      this.group.add(mesh);
      mesh.position.copy(home);
      this.slabs.push({ mesh, section, home, state: 0, dropping: 0, spin: new THREE.Vector3() });
    }
    // The core stands on a pillar that runs down out of sight.
    this.core = new THREE.Mesh(
      new THREE.CylinderGeometry(setup.core, setup.core * 0.55, 14, 40),
      new THREE.MeshStandardMaterial({ color: '#b9c0cf', roughness: 0.85 }),
    );
    this.core.position.y = -7;
    this.core.receiveShadow = true;
    this.group.add(this.core);
    const half = Math.acos(Math.max(-1, Math.min(1, setup.cone)));
    // A fan pointing down +z, the way a rider faces once turned by yaw.
    this.wedgeGeometry = new THREE.CircleGeometry(setup.reach, 20, Math.PI / 2 - half, half * 2);
    this.wedgeGeometry.rotateX(-Math.PI / 2);
    this.wedgeGeometry.rotateY(Math.PI);
  }

  private marker(seat: number) {
    let marker = this.markers.get(seat);
    if (!marker && this.wedgeGeometry) {
      const wedge = new THREE.Mesh(
        this.wedgeGeometry,
        new THREE.MeshBasicMaterial({ color: WINDUP, transparent: true, opacity: 0, depthWrite: false, side: THREE.DoubleSide }),
      );
      wedge.renderOrder = 5;
      const brace = new THREE.Mesh(
        this.braceGeometry,
        new THREE.MeshBasicMaterial({ color: BRACE, transparent: true, opacity: 0.8, depthWrite: false, side: THREE.DoubleSide }),
      );
      brace.rotation.x = -Math.PI / 2;
      brace.renderOrder = 6;
      this.group.add(wedge, brace);
      marker = { wedge, brace, windupAt: null };
      this.markers.set(seat, marker);
    }
    return marker;
  }

  private wave(x: number, z: number, color: string, grow: number) {
    const mesh = new THREE.Mesh(
      this.waveGeometry,
      new THREE.MeshBasicMaterial({ color, transparent: true, opacity: 0.9, depthWrite: false, side: THREE.DoubleSide }),
    );
    mesh.rotation.x = -Math.PI / 2;
    mesh.position.set(x, 0.08, z);
    this.group.add(mesh);
    this.waves.push({ mesh, life: 0.45, grow });
  }

  effect(event: GameEvent, time: number) {
    if (event.kind === 'ledge_windup') {
      const marker = this.marker(event.seat);
      if (marker) marker.windupAt = time;
    } else if (event.kind === 'ledge_shove') {
      const marker = this.markers.get(event.seat);
      if (marker) marker.windupAt = null;
      if (!event.landed && !event.blocked) this.wave(event.x + Math.sin(event.yaw) * 1.2, event.z + Math.cos(event.yaw) * 1.2, '#fde68a', 1.6);
    } else if (event.kind === 'ledge_hit') {
      this.wave(event.x, event.z, '#fb7185', 3.2);
      this.shake = Math.max(this.shake, 0.22);
    } else if (event.kind === 'ledge_block') {
      this.wave(event.x, event.z, BRACE, 2.2);
      this.shake = Math.max(this.shake, 0.1);
    } else if (event.kind === 'ledge_fall') {
      this.shake = Math.max(this.shake, 0.18);
    }
  }

  update(snap: ArenaSnap | null, mySeat: number | null, time: number, dt: number) {
    this.shake = Math.max(0, this.shake - dt * 1.6);
    const setup = this.setup;
    if (!setup || !snap) return;
    const footing = snap.sections ?? [];
    for (const slab of this.slabs) {
      const state = footing[slab.section.index] ?? 0;
      if (state !== slab.state) {
        if (state >= 2 && slab.state < 2) {
          slab.dropping = 0;
          slab.spin.set(Math.random() - 0.5, Math.random() - 0.5, Math.random() - 0.5).multiplyScalar(1.6);
        }
        slab.state = state;
      }
      const { mesh } = slab;
      if (state === 0) {
        mesh.visible = true;
        mesh.position.copy(slab.home);
        mesh.rotation.set(0, 0, 0);
        mesh.material.color.set(STONE[slab.section.ring % STONE.length]);
        mesh.material.emissive.setRGB(0, 0, 0);
      } else if (state === 1) {
        // Shaking harder and glowing hotter the longer it has been cracking.
        mesh.visible = true;
        const jitter = 0.05 + 0.04 * Math.sin(time * 3);
        mesh.position.set(
          slab.home.x + (Math.random() - 0.5) * jitter,
          (Math.random() - 0.5) * jitter * 0.6,
          slab.home.z + (Math.random() - 0.5) * jitter,
        );
        mesh.material.color.set(STONE[slab.section.ring % STONE.length]).lerp(CRACK, 0.55);
        mesh.material.emissive.copy(CRACK).multiplyScalar(0.25 + 0.25 * Math.abs(Math.sin(time * 9)));
      } else {
        slab.dropping += dt;
        const t = slab.dropping;
        mesh.position.set(slab.home.x, -11 * t * t, slab.home.z);
        mesh.rotation.set(slab.spin.x * t, slab.spin.y * t, slab.spin.z * t);
        mesh.visible = t < 2.2;
      }
    }
    if (this.core) {
      const worn = Math.max(0.05, (snap.core ?? setup.core) / setup.core);
      this.core.scale.set(worn, 1, worn);
      const wearing = (snap.core ?? setup.core) < setup.core - 0.01;
      this.core.material.emissive.set(wearing ? '#7c2d12' : '#000000');
      this.core.material.emissiveIntensity = wearing ? 0.3 + 0.2 * Math.abs(Math.sin(time * 4)) : 0;
    }

    for (const player of snap.players) this.mark(player, player.seat === mySeat, time);
    for (const [seat, marker] of this.markers) {
      if (snap.players.some((player) => player.seat === seat)) continue;
      marker.wedge.visible = false;
      marker.brace.visible = false;
    }

    for (let i = this.waves.length - 1; i >= 0; i--) {
      const wave = this.waves[i];
      wave.life -= dt;
      const grown = 1 + (0.45 - wave.life) * wave.grow * 2;
      wave.mesh.scale.setScalar(grown);
      wave.mesh.material.opacity = Math.max(0, wave.life * 2);
      if (wave.life <= 0) {
        wave.mesh.material.dispose();
        wave.mesh.removeFromParent();
        this.waves.splice(i, 1);
      }
    }
  }

  private mark(player: ArenaPlayer, mine: boolean, time: number) {
    const marker = this.marker(player.seat);
    if (!marker || !this.setup) return;
    const yaw = player.yaw ?? 0;
    marker.wedge.position.set(player.x, 0.06, player.z);
    marker.wedge.rotation.y = yaw;
    if (player.out) {
      marker.wedge.visible = false;
      marker.brace.visible = false;
      return;
    }
    if (player.winding) {
      const progress = marker.windupAt === null ? 1 : Math.min(1, (time - marker.windupAt) / this.setup.windup);
      marker.wedge.visible = true;
      marker.wedge.material.color.set(WINDUP);
      marker.wedge.material.opacity = 0.25 + 0.5 * progress;
    } else if (mine && player.ready && !player.skidding) {
      // Your own reach, faintly, so you know where a shove will go.
      marker.wedge.visible = true;
      marker.wedge.material.color.set('#fef3c7');
      marker.wedge.material.opacity = 0.14;
    } else {
      marker.wedge.visible = false;
    }
    marker.brace.visible = !!player.braced;
    marker.brace.position.set(player.x, 0.07, player.z);
    marker.brace.material.opacity = 0.55 + 0.25 * Math.sin(time * 5);
  }

  private clear() {
    for (const slab of this.slabs) {
      slab.mesh.geometry.dispose();
      slab.mesh.material.dispose();
      slab.mesh.removeFromParent();
    }
    this.slabs = [];
    if (this.core) {
      this.core.geometry.dispose();
      this.core.material.dispose();
      this.core.removeFromParent();
      this.core = null;
    }
    for (const marker of this.markers.values()) {
      marker.wedge.material.dispose();
      marker.brace.material.dispose();
      marker.wedge.removeFromParent();
      marker.brace.removeFromParent();
    }
    this.markers.clear();
    for (const wave of this.waves) {
      wave.mesh.material.dispose();
      wave.mesh.removeFromParent();
    }
    this.waves = [];
    this.wedgeGeometry?.dispose();
    this.wedgeGeometry = null;
    this.setup = null;
  }

  dispose() {
    this.clear();
    this.braceGeometry.dispose();
    this.waveGeometry.dispose();
    this.group.removeFromParent();
  }
}
