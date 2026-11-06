/**
 * The night look: bloom on bright things, fake light cones and pools that
 * cost one draw call each, and a small fixed set of real lights (so shaders
 * never recompile) that follow whatever needs them most.
 */

import * as THREE from 'three';
import { EffectComposer } from 'three/examples/jsm/postprocessing/EffectComposer.js';
import { OutputPass } from 'three/examples/jsm/postprocessing/OutputPass.js';
import { RenderPass } from 'three/examples/jsm/postprocessing/RenderPass.js';
import { UnrealBloomPass } from 'three/examples/jsm/postprocessing/UnrealBloomPass.js';

let poolTexture: THREE.Texture | null = null;

/** A soft round falloff, drawn once into pixels. */
export function glowTexture(): THREE.Texture {
  if (poolTexture) return poolTexture;
  const size = 64;
  const data = new Uint8Array(size * size * 4);
  for (let y = 0; y < size; y++) {
    for (let x = 0; x < size; x++) {
      const d = Math.hypot(x - size / 2 + 0.5, y - size / 2 + 0.5) / (size / 2);
      const a = Math.max(0, 1 - d) ** 2;
      data.set([255, 255, 255, Math.round(a * 255)], (y * size + x) * 4);
    }
  }
  const t = new THREE.DataTexture(data, size, size);
  t.magFilter = THREE.LinearFilter;
  t.minFilter = THREE.LinearFilter;
  t.needsUpdate = true;
  poolTexture = t;
  return t;
}

export function glowMaterial(color: THREE.ColorRepresentation, opacity = 1) {
  return new THREE.MeshBasicMaterial({
    map: glowTexture(),
    color,
    transparent: true,
    opacity,
    depthWrite: false,
    blending: THREE.AdditiveBlending,
  });
}

/**
 * Two headlight beams and the patch of road they light, as one mesh. The
 * beam fades from the lamp to its far end through vertex colours.
 */
export function beamGeometry(width: number, reach = 26): THREE.BufferGeometry {
  const parts: THREE.BufferGeometry[] = [];
  for (const side of [-1, 1]) {
    const cone = new THREE.ConeGeometry(3.2, reach, 10, 1, true);
    cone.translate(0, -reach / 2, 0);
    // Apex at the lamp, opening forward along -z and dipping a little toward the road.
    cone.rotateX(Math.PI / 2 - 0.07);
    cone.translate(side * width * 0.33, 0.75, -0.1);
    const pos = cone.getAttribute('position');
    const colors = new Float32Array(pos.count * 3);
    for (let i = 0; i < pos.count; i++) {
      const k = Math.max(0, 1 + pos.getZ(i) / reach) ** 1.8 * 0.05;
      colors.set([k, k * 0.96, k * 0.85], i * 3);
    }
    cone.setAttribute('color', new THREE.BufferAttribute(colors, 3));
    parts.push(cone.toNonIndexed());
  }
  const pool = new THREE.PlaneGeometry(8, 16, 8, 12).rotateX(-Math.PI / 2).translate(0, 0.12, -9);
  const ppos = pool.getAttribute('position');
  const pcol = new Float32Array(ppos.count * 3);
  for (let i = 0; i < ppos.count; i++) {
    // A soft oval, brightest a few meters ahead of the bumper.
    const u = ppos.getX(i) / 4;
    const v = (ppos.getZ(i) + 7) / 8;
    const k = Math.max(0, 1 - Math.hypot(u, v)) ** 1.5 * 0.2;
    pcol.set([k, k * 0.95, k * 0.82], i * 3);
  }
  pool.setAttribute('color', new THREE.BufferAttribute(pcol, 3));
  parts.push(pool.toNonIndexed());
  const merged = new THREE.BufferGeometry();
  const pos = parts.flatMap((p) => Array.from(p.getAttribute('position').array as Float32Array));
  const col = parts.flatMap((p) => Array.from(p.getAttribute('color').array as Float32Array));
  merged.setAttribute('position', new THREE.Float32BufferAttribute(pos, 3));
  merged.setAttribute('color', new THREE.Float32BufferAttribute(col, 3));
  return merged;
}

export const beamMaterial = () =>
  new THREE.MeshBasicMaterial({ vertexColors: true, transparent: true, depthWrite: false, blending: THREE.AdditiveBlending, side: THREE.DoubleSide });

/** A flashlight or spotlight cone. */
export function torchGeometry(reach: number, spread: number): THREE.BufferGeometry {
  const cone = new THREE.ConeGeometry(Math.tan(spread) * reach, reach, 12, 1, true);
  cone.translate(0, -reach / 2, 0);
  cone.rotateX(Math.PI / 2);
  const pos = cone.getAttribute('position');
  const colors = new Float32Array(pos.count * 3);
  for (let i = 0; i < pos.count; i++) {
    const k = Math.max(0, 1 + pos.getZ(i) / reach) ** 1.6 * 0.07;
    colors.set([k, k, k * 0.92], i * 3);
  }
  cone.setAttribute('color', new THREE.BufferAttribute(colors, 3));
  return cone;
}

/** Bloom over the whole frame; skipped on light devices. */
export class Glow {
  composer: EffectComposer;
  bloom: UnrealBloomPass;

  constructor(renderer: THREE.WebGLRenderer, scene: THREE.Scene, camera: THREE.Camera) {
    this.composer = new EffectComposer(renderer);
    this.composer.addPass(new RenderPass(scene, camera));
    const size = renderer.getSize(new THREE.Vector2());
    this.bloom = new UnrealBloomPass(new THREE.Vector2(size.x / 2, size.y / 2), 0.85, 0.45, 0.72);
    this.composer.addPass(this.bloom);
    this.composer.addPass(new OutputPass());
  }

  resize(w: number, h: number) {
    this.composer.setSize(w, h);
    this.bloom.resolution.set(w / 2, h / 2);
  }

  render() {
    this.composer.render();
  }

  dispose() {
    this.composer.dispose();
  }
}

/**
 * The real lights there are: one spot for the player's own headlights, one
 * for a handheld torch or a cruiser's spotlight, and a red and a blue point
 * for the nearest light bar.
 */
export class Lights {
  headlight = new THREE.SpotLight(0xfff1d0, 0, 95, 0.55, 0.5, 1.2);
  torch = new THREE.SpotLight(0xf4f6ff, 0, 60, 0.32, 0.4, 1.3);
  red = new THREE.PointLight(0xff2a2a, 0, 45, 1.6);
  blue = new THREE.PointLight(0x2a5cff, 0, 45, 1.6);

  constructor(scene: THREE.Scene) {
    scene.add(this.headlight, this.headlight.target, this.torch, this.torch.target, this.red, this.blue);
  }

  /** Puts the bar lights on the nearest flashing cruiser, swapping red and blue a few times a second. */
  flash(at: THREE.Vector3 | null, t: number) {
    if (!at) {
      this.red.intensity = 0;
      this.blue.intensity = 0;
      return;
    }
    const phase = Math.floor(t * 6) % 2 === 0;
    this.red.position.set(at.x - 0.4, at.y + 1.9, at.z);
    this.blue.position.set(at.x + 0.4, at.y + 1.9, at.z);
    this.red.intensity = phase ? 80 : 6;
    this.blue.intensity = phase ? 6 : 80;
  }
}
