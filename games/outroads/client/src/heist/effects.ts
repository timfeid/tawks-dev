/** Sparks off bare rims and smoke from a hurt engine, from small pooled particle sets. */

import * as THREE from 'three';
import { glowTexture } from './night';

type Particle = { x: number; y: number; z: number; vx: number; vy: number; vz: number; life: number; max: number };

class Pool {
  points: THREE.Points;
  private parts: Particle[] = [];
  private positions: Float32Array;
  private sizes: Float32Array;

  constructor(
    private capacity: number,
    material: THREE.PointsMaterial,
    private gravity: number,
  ) {
    this.positions = new Float32Array(capacity * 3);
    this.sizes = new Float32Array(capacity);
    const g = new THREE.BufferGeometry();
    g.setAttribute('position', new THREE.BufferAttribute(this.positions, 3));
    this.points = new THREE.Points(g, material);
    this.points.frustumCulled = false;
  }

  emit(x: number, y: number, z: number, vx: number, vy: number, vz: number, life: number) {
    if (this.parts.length >= this.capacity) this.parts.shift();
    this.parts.push({ x, y, z, vx, vy, vz, life, max: life });
  }

  update(dt: number) {
    this.parts = this.parts.filter((p) => (p.life -= dt) > 0);
    for (const p of this.parts) {
      p.vy -= this.gravity * dt;
      p.x += p.vx * dt;
      p.y += p.vy * dt;
      p.z += p.vz * dt;
    }
    this.positions.fill(0);
    this.parts.forEach((p, i) => this.positions.set([p.x, p.y, p.z], i * 3));
    const g = this.points.geometry;
    g.setDrawRange(0, this.parts.length);
    g.getAttribute('position').needsUpdate = true;
    void this.sizes;
  }
}

export class Effects {
  sparks: Pool;
  smoke: Pool;

  constructor(scene: THREE.Scene) {
    this.sparks = new Pool(
      240,
      new THREE.PointsMaterial({ color: 0xffb04a, size: 0.22, map: glowTexture(), transparent: true, depthWrite: false, blending: THREE.AdditiveBlending }),
      9,
    );
    this.smoke = new Pool(160, new THREE.PointsMaterial({ color: 0x5c5e63, size: 1.6, map: glowTexture(), transparent: true, opacity: 0.45, depthWrite: false }), -0.6);
    scene.add(this.sparks.points, this.smoke.points);
  }

  /** Sparks from a wheel spot at a speed, smoke from a bonnet. */
  rim(x: number, y: number, z: number, h: number, speed: number, dt: number) {
    const n = Math.min(6, Math.ceil(Math.abs(speed) * dt * 4));
    for (let i = 0; i < n; i++) {
      const back = -Math.sign(speed) * (2 + Math.random() * 4);
      this.sparks.emit(x, y + 0.05, z, Math.sin(h) * back + (Math.random() - 0.5) * 3, 1 + Math.random() * 2.5, -Math.cos(h) * back + (Math.random() - 0.5) * 3, 0.25 + Math.random() * 0.25);
    }
  }

  engine(x: number, y: number, z: number, dt: number) {
    if (Math.random() < dt * 14) this.smoke.emit(x + (Math.random() - 0.5) * 0.6, y, z + (Math.random() - 0.5) * 0.6, (Math.random() - 0.5) * 0.4, 0.6, (Math.random() - 0.5) * 0.4, 1.8);
  }

  update(dt: number) {
    this.sparks.update(dt);
    this.smoke.update(dt);
  }
}
