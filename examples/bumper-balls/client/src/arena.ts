import * as THREE from 'three';
import type { BoutFrame, BoutHost, BoutHud, BoutRenderer, Character, Pose, Rider, Seat, Shot } from '@tawks/bout-sdk';
import type { BallSnap, BumperEvent, BumperSnap } from './snapshot';

const ARENA_RADIUS = 9;
const RIDER_SCALE = 0.95;

type BallView = {
  seat: number;
  characterId: string;
  name: string;
  ball: THREE.Mesh<THREE.SphereGeometry, THREE.MeshPhysicalMaterial>;
  rider: Rider;
  tag: THREE.Sprite;
  last: THREE.Vector3 | null;
  yaw: number;
  radius: number;
};

type Particle = { sprite: THREE.Sprite; velocity: THREE.Vector3; life: number; maxLife: number; gravity: number; grow: number };

function canvasTexture(width: number, height: number, draw: (ctx: CanvasRenderingContext2D) => void) {
  const canvas = document.createElement('canvas');
  canvas.width = width;
  canvas.height = height;
  draw(canvas.getContext('2d')!);
  const texture = new THREE.CanvasTexture(canvas);
  texture.colorSpace = THREE.SRGBColorSpace;
  return texture;
}

function platformTop() {
  return canvasTexture(1024, 1024, (ctx) => {
    const c = 512;
    const wedges = 16;
    for (let i = 0; i < wedges; i++) {
      ctx.beginPath();
      ctx.moveTo(c, c);
      ctx.arc(c, c, c, (i / wedges) * Math.PI * 2, ((i + 1) / wedges) * Math.PI * 2);
      ctx.closePath();
      ctx.fillStyle = i % 2 ? '#ff5f6d' : '#fff3d1';
      ctx.fill();
    }
    ctx.lineWidth = 34;
    ctx.strokeStyle = '#ffc93c';
    ctx.beginPath();
    ctx.arc(c, c, c - 20, 0, Math.PI * 2);
    ctx.stroke();
    ctx.lineWidth = 10;
    ctx.strokeStyle = '#ffffff';
    ctx.beginPath();
    ctx.arc(c, c, c - 48, 0, Math.PI * 2);
    ctx.stroke();
    ctx.fillStyle = '#ffc93c';
    ctx.beginPath();
    ctx.arc(c, c, 120, 0, Math.PI * 2);
    ctx.fill();
    ctx.fillStyle = '#ff8a3d';
    ctx.beginPath();
    ctx.arc(c, c, 70, 0, Math.PI * 2);
    ctx.fill();
  });
}

function ballTexture(def: Character) {
  return canvasTexture(512, 256, (ctx) => {
    const colors = [def.body, '#ffffff', def.accent, '#ffffff', def.body, def.belly];
    const gore = 512 / colors.length;
    colors.forEach((color, i) => {
      ctx.fillStyle = color;
      ctx.fillRect(i * gore, 0, gore + 1, 256);
    });
    ctx.fillStyle = '#ffffff';
    ctx.fillRect(0, 0, 512, 18);
    ctx.fillRect(0, 238, 512, 18);
  });
}

function nameTexture(name: string, color: string) {
  const font = '700 40px system-ui, sans-serif';
  const measure = document.createElement('canvas').getContext('2d')!;
  measure.font = font;
  const width = Math.ceil(measure.measureText(name).width) + 44;
  return canvasTexture(width, 64, (ctx) => {
    ctx.fillStyle = 'rgba(15, 23, 42, 0.78)';
    ctx.beginPath();
    ctx.roundRect(0, 4, width, 56, 28);
    ctx.fill();
    ctx.fillStyle = color;
    ctx.beginPath();
    ctx.arc(26, 32, 9, 0, Math.PI * 2);
    ctx.fill();
    ctx.font = font;
    ctx.textBaseline = 'middle';
    ctx.fillStyle = '#ffffff';
    ctx.fillText(name, 42, 34);
  });
}

function starTexture() {
  return canvasTexture(64, 64, (ctx) => {
    ctx.translate(32, 32);
    ctx.fillStyle = '#fff6a8';
    ctx.strokeStyle = '#ffb703';
    ctx.lineWidth = 4;
    ctx.beginPath();
    for (let i = 0; i < 10; i++) {
      const r = i % 2 ? 11 : 28;
      const a = (i / 10) * Math.PI * 2 - Math.PI / 2;
      ctx.lineTo(Math.cos(a) * r, Math.sin(a) * r);
    }
    ctx.closePath();
    ctx.fill();
    ctx.stroke();
  });
}

function dropTexture() {
  return canvasTexture(32, 32, (ctx) => {
    const g = ctx.createRadialGradient(16, 16, 0, 16, 16, 16);
    g.addColorStop(0, 'rgba(255,255,255,1)');
    g.addColorStop(0.5, 'rgba(210,244,255,0.9)');
    g.addColorStop(1, 'rgba(210,244,255,0)');
    ctx.fillStyle = g;
    ctx.fillRect(0, 0, 32, 32);
  });
}

type View = BoutFrame<BumperSnap, null>;

export class BumperArena implements BoutRenderer<BumperSnap, null, BumperEvent> {
  private group = new THREE.Group();
  private quake = 0;
  private platform: THREE.Group;
  private platformMaterials: THREE.Material[] = [];
  private geometries: THREE.BufferGeometry[] = [];
  private textures: THREE.Texture[] = [];
  private views = new Map<number, BallView>();
  private youRing: THREE.Mesh<THREE.RingGeometry, THREE.MeshBasicMaterial>;
  private particles: Particle[] = [];
  private starMaterial: THREE.SpriteMaterial;
  private dropMaterial: THREE.SpriteMaterial;
  private rings: { mesh: THREE.Mesh<THREE.RingGeometry, THREE.MeshBasicMaterial>; life: number }[] = [];

  constructor(private host: BoutHost) {
    this.group.name = 'bumper-arena';
    host.scene.add(this.group);

    const top = platformTop();
    this.textures.push(top);
    const side = new THREE.MeshToonMaterial({ color: '#3a86ff' });
    const face = new THREE.MeshStandardMaterial({ map: top, roughness: 0.55 });
    const under = new THREE.MeshStandardMaterial({ color: '#2b59c3', roughness: 0.8 });
    this.platformMaterials.push(side, face, under);
    this.platform = new THREE.Group();
    const disc = new THREE.Mesh(this.geometry(new THREE.CylinderGeometry(ARENA_RADIUS, ARENA_RADIUS * 0.96, 0.6, 72)), [side, face, under]);
    disc.position.y = -0.3;
    disc.receiveShadow = true;
    disc.castShadow = true;
    const pillarMaterial = new THREE.MeshStandardMaterial({ color: '#f7d488', roughness: 0.9 });
    this.platformMaterials.push(pillarMaterial);
    const pillar = new THREE.Mesh(this.geometry(new THREE.CylinderGeometry(3.2, 1.6, -host.waterY + 0.4, 24)), pillarMaterial);
    pillar.position.y = (host.waterY - 0.6) / 2;
    pillar.castShadow = true;
    this.platform.add(disc, pillar);
    this.group.add(this.platform);

    this.youRing = new THREE.Mesh(
      this.geometry(new THREE.RingGeometry(1, 1.22, 40)),
      new THREE.MeshBasicMaterial({ color: '#ffffff', transparent: true, opacity: 0.9, depthWrite: false }),
    );
    this.youRing.rotation.x = -Math.PI / 2;
    this.youRing.position.y = 0.02;
    this.youRing.visible = false;
    this.group.add(this.youRing);

    const star = starTexture();
    const drop = dropTexture();
    this.textures.push(star, drop);
    this.starMaterial = new THREE.SpriteMaterial({ map: star, transparent: true, depthWrite: false });
    this.dropMaterial = new THREE.SpriteMaterial({ map: drop, transparent: true, depthWrite: false });
  }

  private geometry<T extends THREE.BufferGeometry>(geometry: T): T {
    this.geometries.push(geometry);
    return geometry;
  }

  frame(view: View) {
    this.sync(view);
    this.animate(view);
  }

  shot(): Shot {
    return { kind: 'frame', half: [11.5, 9.5], pitch: 38, portraitPitch: 52 };
  }

  shake() {
    return this.quake;
  }

  hud(view: View): BoutHud {
    const closing = view.phase === 'playing' && !!view.snapshot && view.snapshot.radius < ARENA_RADIUS - 0.02;
    return {
      objective: 'Outlast the field',
      hint: 'Roll with WASD, the arrow keys or a gamepad. Knock everyone off.',
      alert: closing ? 'The edge is closing in' : null,
    };
  }

  /** Keeps one ball and rider per seat in the snapshot, matching each seat's character. */
  private sync(view: View) {
    const snap = view.snapshot;
    const inSnap = new Set(snap?.balls.map((b) => b.seat) ?? []);
    for (const [seat, ball] of this.views) {
      const member = view.seats.find((m) => m.seat === seat);
      if (!inSnap.has(seat) || !member || member.character !== ball.characterId) {
        this.removeView(ball);
        this.views.delete(seat);
      } else if (member.name !== ball.name) {
        this.retag(ball, member, this.host.character(member.character));
      }
    }
    for (const ball of snap?.balls ?? []) {
      if (this.views.has(ball.seat)) continue;
      const member = view.seats.find((m) => m.seat === ball.seat);
      const character = member && this.host.character(member.character);
      if (!member || !character) continue;
      this.views.set(ball.seat, this.createView(ball.seat, member, character, snap!.ball_radius));
    }
  }

  private createView(seat: number, member: Seat, character: Character, radius: number): BallView {
    const texture = ballTexture(character);
    const ball = new THREE.Mesh(
      new THREE.SphereGeometry(radius, 36, 24),
      new THREE.MeshPhysicalMaterial({ map: texture, roughness: 0.3, clearcoat: 0.8, clearcoatRoughness: 0.2 }),
    );
    ball.castShadow = true;
    const rider = this.host.rider(character);
    rider.group.scale.setScalar(RIDER_SCALE);
    const tag = new THREE.Sprite(new THREE.SpriteMaterial({ transparent: true, depthWrite: false, depthTest: false }));
    tag.renderOrder = 20;
    this.group.add(ball, rider.group, tag);
    const view: BallView = { seat, characterId: character.id, name: '', ball, rider, tag, last: null, yaw: 0, radius };
    this.retag(view, member, character);
    return view;
  }

  private retag(view: BallView, member: Seat, character: Character | undefined) {
    view.name = member.name;
    const material = view.tag.material;
    material.map?.dispose();
    const texture = nameTexture(member.name, character?.body ?? '#ffffff');
    material.map = texture;
    material.needsUpdate = true;
    const image = texture.image as HTMLCanvasElement;
    view.tag.scale.set((image.width / image.height) * 0.62, 0.62, 1);
  }

  private removeView(view: BallView) {
    view.ball.geometry.dispose();
    view.ball.material.map?.dispose();
    view.ball.material.dispose();
    view.tag.material.map?.dispose();
    view.tag.material.dispose();
    view.ball.removeFromParent();
    view.tag.removeFromParent();
    view.rider.dispose();
  }

  private animate(view: View) {
    const { snapshot: snap, time, dt } = view;
    const radius = snap?.radius ?? ARENA_RADIUS;
    this.platform.scale.set(radius / ARENA_RADIUS, 1, radius / ARENA_RADIUS);
    let mine: THREE.Vector3 | null = null;
    for (const ballSnap of snap?.balls ?? []) {
      const center = this.place(ballSnap, view);
      if (center && ballSnap.seat === view.mySeat && ballSnap.state === 'rolling') mine = center;
    }

    this.youRing.visible = !!mine;
    if (mine) {
      this.youRing.position.set(mine.x, 0.03, mine.z);
      this.youRing.scale.setScalar(1 + Math.sin(time * 5) * 0.06);
      this.youRing.material.opacity = 0.65 + Math.sin(time * 5) * 0.2;
    }

    for (let i = this.particles.length - 1; i >= 0; i--) {
      const p = this.particles[i];
      p.life -= dt;
      if (p.life <= 0) {
        p.sprite.removeFromParent();
        p.sprite.material.dispose();
        this.particles.splice(i, 1);
        continue;
      }
      p.velocity.y -= p.gravity * dt;
      p.sprite.position.addScaledVector(p.velocity, dt);
      p.sprite.material.opacity = Math.min(1, (p.life / p.maxLife) * 1.6);
      p.sprite.scale.setScalar(p.sprite.scale.x + p.grow * dt);
    }
    for (let i = this.rings.length - 1; i >= 0; i--) {
      const ring = this.rings[i];
      ring.life -= dt;
      if (ring.life <= 0) {
        ring.mesh.removeFromParent();
        ring.mesh.geometry.dispose();
        ring.mesh.material.dispose();
        this.rings.splice(i, 1);
        continue;
      }
      ring.mesh.scale.setScalar(ring.mesh.scale.x + dt * 3);
      ring.mesh.material.opacity = ring.life * ring.life;
    }
    this.quake = Math.max(0, this.quake - dt * 3);
  }

  /** Rolls one ball to where the snapshot has it and poses its rider. */
  private place(ballSnap: BallSnap, frame: View): THREE.Vector3 | null {
    const view = this.views.get(ballSnap.seat);
    if (!view) return null;
    const out = ballSnap.state === 'out';
    view.ball.visible = !out;
    view.rider.group.visible = !out;
    view.tag.visible = !out;
    if (out) {
      view.last = null;
      return null;
    }
    const center = new THREE.Vector3(ballSnap.x, ballSnap.y + view.radius, ballSnap.z);
    if (view.last) {
      const dx = center.x - view.last.x;
      const dz = center.z - view.last.z;
      const distance = Math.hypot(dx, dz);
      if (distance > 1e-5 && distance < 2) {
        const axis = new THREE.Vector3(dz, 0, -dx).normalize();
        view.ball.quaternion.premultiply(new THREE.Quaternion().setFromAxisAngle(axis, distance / view.radius));
      }
    }
    view.last = center.clone();
    view.ball.position.copy(center);

    const speed = Math.hypot(ballSnap.vx, ballSnap.vz);
    const turnTo = (target: number, rate: number) => {
      const delta = Math.atan2(Math.sin(target - view.yaw), Math.cos(target - view.yaw));
      view.yaw += delta * Math.min(1, frame.dt * rate);
    };
    if (speed > 0.4) turnTo(Math.atan2(ballSnap.vx, ballSnap.vz), 8);
    else if (frame.phase === 'lobby' || frame.phase === 'countdown') turnTo(Math.atan2(-ballSnap.x, 24 - ballSnap.z), 4);
    view.rider.group.position.set(center.x, center.y + view.radius - 0.08, center.z);
    view.rider.group.rotation.y = view.yaw;
    let pose: Pose = 'idle';
    if (ballSnap.state === 'falling') pose = 'fall';
    else if (frame.phase === 'playing') pose = 'ride';
    else if (frame.phase === 'results') pose = frame.winners.includes(ballSnap.seat) ? 'cheer' : 'sad';
    view.rider.animate(pose, frame.time + ballSnap.seat * 0.37, speed);
    view.tag.position.set(center.x, center.y + view.radius + 2.05, center.z);
    return center;
  }

  effect(event: BumperEvent) {
    if (event.kind === 'bump') {
      this.host.sounds.bump(event.strength);
      this.burstStars(event.x, event.z, event.strength);
      this.quake = Math.min(0.5, this.quake + event.strength * 0.18);
    } else if (event.kind === 'fall') {
      this.host.sounds.fall();
    } else if (event.kind === 'splash') {
      this.host.sounds.splash();
      this.splash(event.x, event.z);
    }
  }

  private burstStars(x: number, z: number, strength: number) {
    const count = 5 + Math.round(strength * 4);
    for (let i = 0; i < count; i++) {
      const angle = (i / count) * Math.PI * 2 + Math.random() * 0.5;
      const speed = 3 + strength * 3;
      this.spawn(this.starMaterial, new THREE.Vector3(x, 1.1, z), new THREE.Vector3(Math.cos(angle) * speed, 2 + Math.random() * 2, Math.sin(angle) * speed), 0.45, 0.4, 6, -0.4);
    }
  }

  private splash(x: number, z: number) {
    const { waterY } = this.host;
    for (let i = 0; i < 18; i++) {
      const angle = Math.random() * Math.PI * 2;
      const speed = 1.5 + Math.random() * 3;
      this.spawn(this.dropMaterial, new THREE.Vector3(x, waterY + 0.2, z), new THREE.Vector3(Math.cos(angle) * speed, 5 + Math.random() * 5, Math.sin(angle) * speed), 0.9, 0.35, 18, 0);
    }
    const ring = new THREE.Mesh(
      new THREE.RingGeometry(0.7, 0.85, 32),
      new THREE.MeshBasicMaterial({ color: '#ffffff', transparent: true, opacity: 1, depthWrite: false, side: THREE.DoubleSide }),
    );
    ring.rotation.x = -Math.PI / 2;
    ring.position.set(x, waterY + 0.25, z);
    this.group.add(ring);
    this.rings.push({ mesh: ring, life: 0.8 });
  }

  private spawn(material: THREE.SpriteMaterial, at: THREE.Vector3, velocity: THREE.Vector3, life: number, size: number, gravity: number, grow: number) {
    const sprite = new THREE.Sprite(material.clone());
    sprite.position.copy(at);
    sprite.scale.setScalar(size);
    this.group.add(sprite);
    this.particles.push({ sprite, velocity, life, maxLife: life, gravity, grow });
  }

  dispose() {
    this.views.forEach((view) => this.removeView(view));
    this.views.clear();
    this.particles.forEach((p) => p.sprite.material.dispose());
    this.rings.forEach((r) => {
      r.mesh.geometry.dispose();
      r.mesh.material.dispose();
    });
    this.geometries.forEach((g) => g.dispose());
    this.platformMaterials.forEach((m) => m.dispose());
    this.youRing.material.dispose();
    this.starMaterial.dispose();
    this.dropMaterial.dispose();
    this.textures.forEach((t) => t.dispose());
    this.group.removeFromParent();
  }
}
