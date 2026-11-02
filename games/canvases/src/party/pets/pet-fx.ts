import * as THREE from 'three';
import { FxSim, GLYPHS, SPRITE_SCALE, ambienceFor, fade, moveBurst, sizeOf, type FxName, type Glyph } from './fx-sim';

const CELL = 64;
let atlas: THREE.CanvasTexture | null = null;

/** Each glyph drawn white in one strip, tinted per particle. */
function glyphAtlas() {
  if (atlas) return atlas;
  const canvas = document.createElement('canvas');
  canvas.width = CELL * GLYPHS.length;
  canvas.height = CELL;
  const g = canvas.getContext('2d')!;
  g.fillStyle = '#fff';
  g.strokeStyle = '#fff';
  const draw: Record<Glyph, () => void> = {
    heart: () => {
      g.beginPath();
      g.moveTo(32, 54);
      g.bezierCurveTo(4, 34, 8, 8, 32, 20);
      g.bezierCurveTo(56, 8, 60, 34, 32, 54);
      g.fill();
    },
    star: () => {
      g.beginPath();
      for (let i = 0; i < 10; i++) {
        const r = i % 2 ? 11 : 27;
        const a = (i / 10) * Math.PI * 2 - Math.PI / 2;
        g.lineTo(32 + Math.cos(a) * r, 33 + Math.sin(a) * r);
      }
      g.fill();
    },
    sparkle: () => {
      g.beginPath();
      for (let i = 0; i < 8; i++) {
        const r = i % 2 ? 6 : 28;
        const a = (i / 8) * Math.PI * 2;
        g.lineTo(32 + Math.cos(a) * r, 32 + Math.sin(a) * r);
      }
      g.fill();
    },
    dot: () => {
      const soft = g.createRadialGradient(32, 32, 4, 32, 32, 28);
      soft.addColorStop(0, 'rgba(255,255,255,1)');
      soft.addColorStop(1, 'rgba(255,255,255,0)');
      g.fillStyle = soft;
      g.fillRect(0, 0, CELL, CELL);
      g.fillStyle = '#fff';
    },
    bang: () => {
      g.font = '900 54px system-ui, sans-serif';
      g.textAlign = 'center';
      g.textBaseline = 'middle';
      g.lineWidth = 8;
      g.strokeStyle = 'rgba(0,0,0,0.55)';
      g.strokeText('!', 32, 34);
      g.fillText('!', 32, 34);
      g.strokeStyle = '#fff';
    },
    ring: () => {
      g.lineWidth = 5;
      g.beginPath();
      g.arc(32, 32, 26, 0, Math.PI * 2);
      g.stroke();
    },
    bubble: () => {
      g.lineWidth = 3;
      g.beginPath();
      g.arc(32, 32, 24, 0, Math.PI * 2);
      g.stroke();
      g.beginPath();
      g.arc(24, 23, 5, 0, Math.PI * 2);
      g.fill();
    },
  };
  GLYPHS.forEach((glyph, index) => {
    g.save();
    g.translate(index * CELL, 0);
    draw[glyph]();
    g.restore();
  });
  atlas = new THREE.CanvasTexture(canvas);
  atlas.colorSpace = THREE.SRGBColorSpace;
  return atlas;
}

const VERTEX = `
attribute float size;
attribute float alpha;
attribute float glyph;
attribute vec3 tint;
uniform float scale;
uniform float ortho;
varying float vAlpha;
varying float vGlyph;
varying vec3 vTint;
void main() {
  vec4 view = modelViewMatrix * vec4(position, 1.0);
  gl_PointSize = ortho > 0.5 ? size * scale : size * scale / max(-view.z, 0.01);
  gl_Position = projectionMatrix * view;
  vAlpha = alpha;
  vGlyph = glyph;
  vTint = tint;
}`;
const FRAGMENT = `
uniform sampler2D atlas;
uniform float cells;
varying float vAlpha;
varying float vGlyph;
varying vec3 vTint;
void main() {
  vec2 uv = vec2((vGlyph + gl_PointCoord.x) / cells, 1.0 - gl_PointCoord.y);
  vec4 texel = texture2D(atlas, uv);
  float a = texel.a * vAlpha;
  if (a < 0.01) discard;
  gl_FragColor = vec4(vTint * texel.rgb, a);
}`;

/** Draws an FxSim as camera-facing sprites, with rings lying on the ground and "+N" floating up. */
export class PetFx {
  group = new THREE.Group();
  private sim = new FxSim();
  private points: THREE.Points;
  private geometry = new THREE.BufferGeometry();
  private material: THREE.ShaderMaterial;
  private rings: THREE.Mesh[] = [];
  private labels: { sprite: THREE.Sprite; age: number }[] = [];
  private color = new THREE.Color();

  /** `height` is how tall the pet stands, which every fx.json length is measured in. */
  constructor(height: number) {
    this.group.scale.setScalar(height);
    this.group.name = 'pet-fx';
    const capacity = this.capacity;
    for (const [name, size] of [['position', 3], ['tint', 3], ['size', 1], ['alpha', 1], ['glyph', 1]] as const)
      this.geometry.setAttribute(name, new THREE.BufferAttribute(new Float32Array(capacity * size), size));
    this.geometry.setDrawRange(0, 0);
    this.material = new THREE.ShaderMaterial({
      uniforms: { atlas: { value: glyphAtlas() }, cells: { value: GLYPHS.length }, scale: { value: 400 }, ortho: { value: 0 } },
      vertexShader: VERTEX,
      fragmentShader: FRAGMENT,
      transparent: true,
      depthWrite: false,
    });
    this.points = new THREE.Points(this.geometry, this.material);
    this.points.frustumCulled = false;
    this.points.renderOrder = 5;
    this.points.onBeforeRender = (renderer, _scene, camera) => {
      const tall = renderer.getDrawingBufferSize(new THREE.Vector2()).y;
      // Sizes are in pet heights; the group's scale reaches the vertex shader through the model-view matrix, not the point size.
      const ortho = camera instanceof THREE.OrthographicCamera;
      const across = ortho ? (camera.top - camera.bottom) / camera.zoom : 2 * Math.tan(THREE.MathUtils.degToRad((camera as THREE.PerspectiveCamera).fov / 2));
      this.material.uniforms.ortho.value = ortho ? 1 : 0;
      this.material.uniforms.scale.value = (tall / across) * this.group.scale.x;
    };
    this.group.add(this.points);
  }

  private get capacity() {
    return 160;
  }

  setAmbience(stage: number, effect: string | null | undefined) {
    this.sim.setAmbient(ambienceFor(stage, effect));
  }

  burst(name: FxName | string) {
    const burst = moveBurst(name);
    if (burst) this.sim.burst(burst);
  }

  /** "+10" rising over the pet, with a sparkle. */
  gain(points: number) {
    this.sim.burst('gain');
    const canvas = document.createElement('canvas');
    canvas.width = 160;
    canvas.height = 80;
    const g = canvas.getContext('2d')!;
    g.font = '800 52px system-ui, sans-serif';
    g.textAlign = 'center';
    g.textBaseline = 'middle';
    g.lineWidth = 8;
    g.strokeStyle = 'rgba(15,20,35,0.8)';
    g.strokeText(`+${points}`, 80, 42);
    g.fillStyle = '#ffd23f';
    g.fillText(`+${points}`, 80, 42);
    const texture = new THREE.CanvasTexture(canvas);
    texture.colorSpace = THREE.SRGBColorSpace;
    const sprite = new THREE.Sprite(new THREE.SpriteMaterial({ map: texture, transparent: true, depthWrite: false }));
    sprite.scale.set(0.7, 0.35, 1);
    sprite.renderOrder = 6;
    this.group.add(sprite);
    this.labels.push({ sprite, age: 0 });
  }

  update(dt: number, walking = 0) {
    this.sim.step(dt, walking);
    const particles = this.sim.particles.filter((p) => !p.flat);
    const flat = this.sim.particles.filter((p) => p.flat);
    const attr = (name: string) => this.geometry.getAttribute(name) as THREE.BufferAttribute;
    const [position, tint, size, alpha, glyph] = ['position', 'tint', 'size', 'alpha', 'glyph'].map(attr);
    const shown = Math.min(particles.length, this.capacity);
    for (let i = 0; i < shown; i++) {
      const p = particles[i];
      position.setXYZ(i, p.x, p.y, p.z);
      // Written straight to the screen, so the colour stays as fx.json says.
      this.color.set(p.color);
      tint.setXYZ(i, this.color.r, this.color.g, this.color.b);
      size.setX(i, sizeOf(p) * SPRITE_SCALE);
      alpha.setX(i, fade(p));
      glyph.setX(i, GLYPHS.indexOf(p.glyph));
    }
    for (const a of [position, tint, size, alpha, glyph]) a.needsUpdate = true;
    this.geometry.setDrawRange(0, shown);

    while (this.rings.length < flat.length) {
      const ring = new THREE.Mesh(
        new THREE.RingGeometry(0.42, 0.5, 48),
        new THREE.MeshBasicMaterial({ transparent: true, depthWrite: false, side: THREE.DoubleSide }),
      );
      ring.rotation.x = -Math.PI / 2;
      ring.renderOrder = 4;
      this.group.add(ring);
      this.rings.push(ring);
    }
    this.rings.forEach((ring, index) => {
      const p = flat[index];
      ring.visible = !!p;
      if (!p) return;
      const material = ring.material as THREE.MeshBasicMaterial;
      material.color.set(p.color);
      material.opacity = fade(p);
      ring.position.set(p.x, p.y, p.z);
      ring.scale.setScalar(sizeOf(p));
    });

    for (const label of this.labels) {
      label.age += dt;
      label.sprite.position.set(0, 1.2 + label.age * 0.45, 0);
      (label.sprite.material as THREE.SpriteMaterial).opacity = Math.min(1, (1.4 - label.age) / 0.5);
    }
    for (const done of this.labels.filter((label) => label.age >= 1.4)) {
      done.sprite.removeFromParent();
      done.sprite.material.map?.dispose();
      done.sprite.material.dispose();
    }
    this.labels = this.labels.filter((label) => label.age < 1.4);
  }

  dispose() {
    this.group.removeFromParent();
    this.geometry.dispose();
    this.material.dispose();
    for (const ring of this.rings) {
      ring.geometry.dispose();
      (ring.material as THREE.Material).dispose();
    }
    for (const label of this.labels) {
      label.sprite.material.map?.dispose();
      label.sprite.material.dispose();
    }
  }
}
