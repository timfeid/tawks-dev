import * as THREE from 'three';
import type { Camera, Color, Effect, Entity, Ground, Seat, Sound, Stage } from './contract';
import type { Sample } from './scene-buffer';
import { createPartyStage, WATER_Y, type PartyStage } from './party/stage';
import { buildCharacter, type CharacterRig } from './party/characters';
import { characterFor } from './party/roster';
import type { CharacterDef, Costume } from './party/types';
import { CameraFlow } from './party/camera-flow';
import { FrameBudget } from './party/frame-budget';
import { PartySfx } from './party/sfx';

export const PALETTE: Record<Color, string> = {
  ice: '#dff3ff',
  water: '#1b86c4',
  danger: '#ef4444',
  seat: '#f5c542',
  gold: '#f5c542',
  wood: '#a86b3c',
  snow: '#f7fbff',
  stone: '#a3acb7',
  grass: '#5cc26b',
  sand: '#f3d99a',
  lava: '#ff5a1f',
  light: '#f8fafc',
  dark: '#273142',
  red: '#e5484d',
  orange: '#f97316',
  yellow: '#facc15',
  green: '#22c55e',
  blue: '#3b82f6',
  purple: '#8b5cf6',
  pink: '#ec4899',
};

const GROUNDS: Record<Ground, string> = { grass: '#5cc26b', stone: '#a3acb7', snow: '#f2f7fb', wood: '#b07a48', sand: '#f3d99a' };

/** Ice, snow and lava catch the light a little; the rest stay matte. */
const GLOW: Partial<Record<Color, number>> = { ice: 0.06, water: 0.05, snow: 0.2, lava: 0.8 };

export type RendererOptions = {
  mobile?: boolean;
  /** Plays sounds for `v.sound` effects. The screenshot page leaves it off. */
  sounds?: boolean;
  /** Keeps the drawn image readable after the frame, for screenshots. */
  preserve?: boolean;
  pixelRatio?: number;
};

type Drawn = {
  kind: Entity['kind'];
  seat?: number;
  object: THREE.Object3D;
  rig?: CharacterRig;
  look?: string;
  tag?: THREE.Sprite;
  /** The ring under the viewer's own character. */
  halo?: THREE.Mesh;
  material?: THREE.Material;
  color?: string;
  fallFrom?: { at: number; y: number } | null;
  yaw: number;
  last?: { x: number; z: number };
};

type Particle = { sprite: THREE.Sprite; velocity: THREE.Vector3; life: number; maxLife: number };

function canvasTexture(width: number, height: number, draw: (ctx: CanvasRenderingContext2D) => void) {
  const canvas = document.createElement('canvas');
  canvas.width = width;
  canvas.height = height;
  draw(canvas.getContext('2d')!);
  const texture = new THREE.CanvasTexture(canvas);
  texture.colorSpace = THREE.SRGBColorSpace;
  return texture;
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

/** The viewer's own tag: a pill in their colour that points down at them. */
function youTexture(color: string) {
  return canvasTexture(160, 96, (ctx) => {
    ctx.fillStyle = color;
    ctx.strokeStyle = 'rgba(15, 23, 42, 0.9)';
    ctx.lineWidth = 5;
    ctx.beginPath();
    ctx.roundRect(4, 4, 152, 60, 30);
    ctx.moveTo(64, 62);
    ctx.lineTo(80, 90);
    ctx.lineTo(96, 62);
    ctx.fill();
    ctx.stroke();
    ctx.fillStyle = color;
    ctx.fillRect(66, 56, 28, 8);
    ctx.font = '900 40px system-ui, sans-serif';
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    ctx.fillStyle = '#0f172a';
    ctx.fillText('YOU', 80, 36);
  });
}

const dropTexture = () =>
  canvasTexture(32, 32, (ctx) => {
    const g = ctx.createRadialGradient(16, 16, 0, 16, 16, 16);
    g.addColorStop(0, 'rgba(255,255,255,1)');
    g.addColorStop(1, 'rgba(255,255,255,0)');
    ctx.fillStyle = g;
    ctx.fillRect(0, 0, 32, 32);
  });

const wrapAngle = (angle: number) => Math.atan2(Math.sin(angle), Math.cos(angle));

/** How tall a seat's character stands, for name tags and the follow camera. */
const TAG_HEIGHT = 1.75;

/**
 * Draws made-kit scenes with the Tawkathon stage and characters. It runs no
 * game code: everything it draws comes from scene frames.
 */
export class MadeRenderer {
  readonly renderer: THREE.WebGLRenderer;
  readonly scene = new THREE.Scene();
  readonly camera = new THREE.PerspectiveCamera(40, 16 / 9, 0.1, 400);
  private stage: PartyStage;
  private sfx: PartySfx | null;
  private drawn = new Map<string, Drawn>();
  private roster = new Map<number, { name: string; def: CharacterDef }>();
  private ballTexture = canvasTexture(128, 64, (ctx) => {
    ctx.fillStyle = '#ffffff';
    ctx.fillRect(0, 0, 128, 64);
    ctx.fillStyle = 'rgba(15, 23, 42, 0.14)';
    for (let i = 0; i < 4; i++) ctx.fillRect(i * 32 + 4, 0, 10, 64);
  });
  private iceTexture = (() => {
    const texture = canvasTexture(512, 512, (ctx) => {
      ctx.fillStyle = '#ffffff';
      ctx.fillRect(0, 0, 512, 512);
      ctx.strokeStyle = 'rgba(120, 190, 230, 0.28)';
      ctx.lineCap = 'round';
      for (let i = 0; i < 40; i++) {
        const x = (i * 137) % 512;
        const y = (i * 251) % 512;
        ctx.lineWidth = 1 + (i % 3);
        ctx.beginPath();
        ctx.moveTo(x, y);
        ctx.lineTo(x + 30 + (i % 7) * 12, y + ((i % 5) - 2) * 14);
        ctx.stroke();
      }
      const shine = ctx.createRadialGradient(200, 180, 10, 256, 256, 300);
      shine.addColorStop(0, 'rgba(255,255,255,0.0)');
      shine.addColorStop(1, 'rgba(170,215,240,0.35)');
      ctx.fillStyle = shine;
      ctx.fillRect(0, 0, 512, 512);
    });
    return texture;
  })();
  private geometry = {
    disc: new THREE.CylinderGeometry(1, 1, 1, 56),
    ball: new THREE.SphereGeometry(1, 32, 20),
    box: new THREE.BoxGeometry(1, 1, 1),
    marker: new THREE.RingGeometry(0.82, 1, 48),
    ground: new THREE.CylinderGeometry(1, 1.08, 1, 64),
  };
  private ground: THREE.Mesh;
  private groundMaterial = new THREE.MeshStandardMaterial({ color: PALETTE.grass, roughness: 1 });
  private flow = new CameraFlow({ duration: 0.48 });
  private budget: FrameBudget;
  private follow = new THREE.Vector3();
  private offset = new THREE.Vector3();
  private lookAt = new THREE.Vector3();
  private snapCamera = true;
  private particles: Particle[] = [];
  private drop = dropTexture();
  private lastShadow = 0;
  private mobile: boolean;
  private ray = new THREE.Raycaster();
  private plane = new THREE.Plane(new THREE.Vector3(0, 1, 0), 0);
  /** The seat this viewer plays, whose name tag is left off. */
  mySeat: number | null = null;

  constructor(
    readonly canvas: HTMLCanvasElement,
    options: RendererOptions = {},
  ) {
    this.mobile = !!options.mobile;
    this.renderer = new THREE.WebGLRenderer({ canvas, antialias: true, preserveDrawingBuffer: !!options.preserve, powerPreference: 'high-performance' });
    this.renderer.outputColorSpace = THREE.SRGBColorSpace;
    this.renderer.toneMapping = THREE.ACESFilmicToneMapping;
    this.renderer.toneMappingExposure = 1.05;
    this.renderer.shadowMap.enabled = true;
    this.renderer.shadowMap.type = THREE.PCFSoftShadowMap;
    this.renderer.shadowMap.autoUpdate = false;
    this.renderer.shadowMap.needsUpdate = true;
    const ratio = options.pixelRatio ?? Math.min(globalThis.devicePixelRatio ?? 1, this.mobile ? 1 : 1.25);
    this.budget = new FrameBudget(ratio);
    this.renderer.setPixelRatio(ratio);
    this.stage = createPartyStage(this.scene, { mobile: this.mobile });
    this.sfx = options.sounds ? new PartySfx() : null;
    this.ground = new THREE.Mesh(this.geometry.ground, this.groundMaterial);
    this.ground.receiveShadow = true;
    this.ground.visible = false;
    this.scene.add(this.ground);
  }

  get muted() {
    return this.sfx?.muted ?? true;
  }

  setMuted(muted: boolean) {
    this.sfx?.setMuted(muted);
  }

  setSuspended(suspended: boolean) {
    this.sfx?.setSuspended(suspended);
  }

  resize(width: number, height: number) {
    if (width <= 0 || height <= 0) return;
    this.renderer.setSize(width, height, false);
    this.camera.aspect = width / height;
    this.camera.updateProjectionMatrix();
  }

  /** Seats and their looks; a changed look rebuilds that character. */
  setRoster(seats: Seat[]) {
    this.roster.clear();
    for (const seat of seats) this.roster.set(seat.seat, { name: seat.name, def: characterFor(seat.costume as Costume | null, seat.seat) });
  }

  seatColor(seat: number) {
    return this.roster.get(seat)?.def.body ?? characterFor(null, seat).body;
  }

  private colorOf(entity: Entity) {
    const color = entity.color ?? (entity.kind === 'ball' ? 'snow' : entity.kind === 'box' ? 'wood' : entity.kind === 'marker' ? 'gold' : 'stone');
    if (color === 'seat' && entity.seat) return this.seatColor(entity.seat);
    return PALETTE[color] ?? PALETTE.stone;
  }

  private material(entity: Entity, color: string) {
    if (entity.kind === 'marker') return new THREE.MeshBasicMaterial({ color, transparent: true, opacity: 0.85, depthWrite: false, side: THREE.DoubleSide });
    const glow = GLOW[entity.color ?? 'stone'];
    const material = new THREE.MeshStandardMaterial({ color, roughness: glow === undefined ? 0.85 : 0.4, emissive: glow ? color : '#000000', emissiveIntensity: glow ?? 0 });
    if (entity.kind === 'ball') material.map = this.ballTexture;
    if (entity.color === 'ice' && entity.kind === 'disc') material.map = this.iceTexture;
    return material;
  }

  private build(entity: Entity): Drawn {
    if (entity.kind === 'player') {
      const seatNumber = entity.seat ?? 1;
      const seat = this.roster.get(seatNumber);
      const def = seat?.def ?? characterFor(null, seatNumber);
      const rig = buildCharacter(def);
      const group = new THREE.Group();
      group.add(rig.group);
      const me = seatNumber === this.mySeat;
      const map = me ? youTexture(def.body) : nameTexture(seat?.name ?? `Seat ${seatNumber}`, def.body);
      const tag = new THREE.Sprite(new THREE.SpriteMaterial({ map, depthTest: false, transparent: true }));
      const image = (tag.material.map!.image as HTMLCanvasElement);
      tag.scale.set((image.width / 64) * 0.42, (image.height / 64) * 0.42, 1);
      tag.position.y = TAG_HEIGHT;
      tag.renderOrder = 10;
      group.add(tag);
      let halo: THREE.Mesh | undefined;
      if (me) {
        halo = new THREE.Mesh(
          new THREE.RingGeometry(0.62, 0.86, 48),
          new THREE.MeshBasicMaterial({ color: def.body, transparent: true, opacity: 0.95, depthWrite: false, side: THREE.DoubleSide }),
        );
        halo.rotation.x = -Math.PI / 2;
        halo.position.y = 0.03;
        halo.renderOrder = 5;
        group.add(halo);
      }
      this.scene.add(group);
      return { kind: 'player', seat: seatNumber, object: group, rig, tag, halo, look: this.lookKey(seatNumber, def), yaw: entity.yaw ?? 0 };
    }
    const color = this.colorOf(entity);
    const material = this.material(entity, color);
    const geometry = entity.kind === 'disc' ? this.geometry.disc : entity.kind === 'box' ? this.geometry.box : entity.kind === 'ball' ? this.geometry.ball : this.geometry.marker;
    const mesh = new THREE.Mesh(geometry, material);
    mesh.castShadow = entity.kind !== 'marker';
    mesh.receiveShadow = true;
    if (entity.kind === 'marker') mesh.rotation.x = -Math.PI / 2;
    this.scene.add(mesh);
    return { kind: entity.kind, object: mesh, material, color, yaw: 0 };
  }

  private release(drawn: Drawn) {
    drawn.object.removeFromParent();
    drawn.rig?.dispose();
    if (drawn.tag) {
      drawn.tag.material.map?.dispose();
      drawn.tag.material.dispose();
    }
    drawn.material?.dispose();
    if (drawn.halo) {
      drawn.halo.geometry.dispose();
      (drawn.halo.material as THREE.Material).dispose();
    }
  }

  private lookKey(seat: number, def: CharacterDef) {
    return `${JSON.stringify(def)}${seat === this.mySeat ? ':me' : ''}`;
  }

  /** The top of whatever a point stands on: the highest disc or box under it, or null over open water. */
  private support(x: number, z: number, entities: Entity[]) {
    let top: number | null = null;
    for (const entity of entities) {
      if (entity.hidden) continue;
      if (entity.kind === 'disc') {
        if (Math.hypot(x - entity.x, z - entity.z) <= (entity.r ?? 1)) top = Math.max(top ?? -Infinity, entity.y + (entity.h ?? 0.2));
      } else if (entity.kind === 'box') {
        const yaw = entity.yaw ?? 0;
        const dx = x - entity.x;
        const dz = z - entity.z;
        const lx = dx * Math.cos(yaw) - dz * Math.sin(yaw);
        const lz = dx * Math.sin(yaw) + dz * Math.cos(yaw);
        if (Math.abs(lx) <= (entity.w ?? 1) / 2 && Math.abs(lz) <= (entity.d ?? 1) / 2) top = Math.max(top ?? -Infinity, entity.y + (entity.h ?? 1));
      }
    }
    return top;
  }

  private placePlayer(drawn: Drawn, entity: Entity, entities: Entity[], speed: number, time: number, dt: number) {
    const group = drawn.object;
    const wanted = entity.pose ?? 'idle';
    // Characters ride when they move; a game that leaves the pose idle still gets the walk.
    const pose = wanted === 'play' ? 'idle' : wanted === 'idle' && speed > 0.6 ? 'ride' : wanted;
    let y = Math.max(entity.y, this.support(entity.x, entity.z, entities) ?? entity.y);
    if (pose === 'fall') {
      drawn.fallFrom ??= { at: time, y: group.position.y || y };
      const t = time - drawn.fallFrom.at;
      y = Math.max(WATER_Y - 1.2, drawn.fallFrom.y - 4.9 * t * t);
    } else drawn.fallFrom = null;
    group.position.set(entity.x, y, entity.z);
    let want = drawn.yaw;
    if (typeof entity.yaw === 'number') want = entity.yaw;
    else if (drawn.last) {
      const dx = entity.x - drawn.last.x;
      const dz = entity.z - drawn.last.z;
      if (Math.hypot(dx, dz) > 0.004) want = Math.atan2(dx, dz);
    }
    drawn.last = { x: entity.x, z: entity.z };
    drawn.yaw += wrapAngle(want - drawn.yaw) * (1 - Math.exp(-dt * 12));
    drawn.rig!.group.rotation.y = drawn.yaw;
    // A character is about 0.84 m wide at scale 1, so a body's radius sets how big it is drawn.
    drawn.rig!.group.scale.setScalar(THREE.MathUtils.clamp((entity.r ?? 0.6) / 0.42, 0.6, 3.5));
    drawn.rig!.animate(pose, time, speed);
    const tag = drawn.tag;
    if (tag) {
      const me = !!drawn.halo;
      tag.visible = pose !== 'fall';
      tag.position.y = TAG_HEIGHT * drawn.rig!.group.scale.y + (me ? 0.25 : 0);
      // Tags keep a readable size on screen however far the camera pulls back.
      const k = THREE.MathUtils.clamp(this.camera.position.distanceTo(group.position) / 22, 0.7, 2.4) * (me ? 1.2 : 1);
      const image = tag.material.map!.image as HTMLCanvasElement;
      tag.scale.set((image.width / 64) * 0.42 * k, (image.height / 64) * 0.42 * k, 1);
    }
    if (drawn.halo) {
      drawn.halo.visible = pose !== 'fall';
      drawn.halo.scale.setScalar(drawn.rig!.group.scale.x * (1 + 0.08 * Math.sin(time * 5)));
    }
  }

  private placeShape(drawn: Drawn, entity: Entity, entities: Entity[]) {
    const color = this.colorOf(entity);
    if (color !== drawn.color && drawn.material) {
      const material = drawn.material as THREE.MeshStandardMaterial;
      material.color.set(color);
      if (material.emissive) material.emissive.set(GLOW[entity.color ?? 'stone'] ? color : '#000000');
      drawn.color = color;
    }
    const mesh = drawn.object;
    const size = (value: number | null | undefined, fallback: number) => Math.max(0.01, value ?? fallback);
    if (entity.kind === 'disc') {
      const h = size(entity.h, 0.2);
      mesh.scale.set(size(entity.r, 1), h, size(entity.r, 1));
      mesh.position.set(entity.x, entity.y + h / 2, entity.z);
    } else if (entity.kind === 'box') {
      const h = size(entity.h, 1);
      mesh.scale.set(size(entity.w, 1), h, size(entity.d, 1));
      mesh.position.set(entity.x, entity.y + h / 2, entity.z);
      mesh.rotation.y = entity.yaw ?? 0;
    } else if (entity.kind === 'ball') {
      const r = size(entity.r, 0.5);
      mesh.scale.setScalar(r);
      mesh.position.set(entity.x, entity.y + r, entity.z);
      mesh.rotation.x = entity.yaw ?? 0;
    } else {
      mesh.scale.set(size(entity.r, 1), size(entity.r, 1), 1);
      mesh.position.set(entity.x, Math.max(entity.y, this.support(entity.x, entity.z, entities) ?? entity.y) + 0.03, entity.z);
    }
  }

  private setStage(stage: Stage, camera: Camera) {
    this.stage.islands.forEach((island) => (island.visible = stage.water));
    this.ground.visible = !!stage.ground;
    if (stage.ground) {
      this.groundMaterial.color.set(GROUNDS[stage.ground] ?? GROUNDS.grass);
      const radius = Math.max(camera.half_x, camera.half_z) * 1.7 + 4;
      this.ground.scale.set(radius, 2, radius);
      this.ground.position.set(camera.x, -1.002, camera.z);
    }
  }

  private frameBox(wide: number, tall: number, pitchDegrees: number) {
    const halfV = THREE.MathUtils.degToRad(this.camera.fov / 2);
    const halfH = Math.atan(Math.tan(halfV) * this.camera.aspect);
    const portrait = this.camera.aspect < 0.8;
    const distance = Math.max(wide / Math.tan(halfH), tall / Math.tan(halfV)) * (portrait ? 1.0 : 1.02);
    const pitch = THREE.MathUtils.degToRad(THREE.MathUtils.clamp(pitchDegrees, 15, 89));
    this.offset.set(0, Math.sin(pitch) * distance, Math.cos(pitch) * distance);
  }

  private placeCamera(camera: Camera, dt: number, now: number) {
    const target = new THREE.Vector3(camera.x, 0, camera.z);
    let key = 'frame';
    this.frameBox(camera.half_x, camera.half_z, camera.pitch);
    if (camera.follow !== null) {
      const drawn = [...this.drawn.values()].find((d) => d.kind === 'player' && d.seat === camera.follow);
      if (drawn) target.set(drawn.object.position.x, Math.max(0, drawn.object.position.y), drawn.object.position.z);
      key = `follow:${camera.follow}`;
    }
    if (this.snapCamera) this.follow.copy(target);
    else this.follow.lerp(target, 1 - Math.exp(-dt * 3.4));
    this.snapCamera = false;
    this.camera.position.copy(this.follow).add(this.offset);
    this.lookAt.set(this.follow.x, this.follow.y + (camera.follow !== null ? 0.6 : -0.4), this.follow.z);
    this.camera.lookAt(this.lookAt);
    this.flow.apply(this.camera, key, now);
    // A wide shot in a narrow pane pulls the camera far back; the stage's fixed
    // fog would wash the whole arena out, so it starts beyond the play area.
    const fog = this.scene.fog as THREE.Fog | null;
    if (fog) {
      const distance = this.offset.length();
      fog.near = Math.max(45, distance * 1.1);
      fog.far = Math.max(120, distance * 2.6);
    }
  }

  private burst(x: number, z: number, color: string) {
    for (let i = 0; i < 18; i++) {
      const material = new THREE.SpriteMaterial({ map: this.drop, color, transparent: true, depthWrite: false });
      const sprite = new THREE.Sprite(material);
      sprite.position.set(x, 0.6, z);
      sprite.scale.setScalar(0.35);
      const angle = (i / 18) * Math.PI * 2;
      const speed = 2.5 + (i % 5) * 0.6;
      this.particles.push({ sprite, velocity: new THREE.Vector3(Math.cos(angle) * speed, 4 + (i % 3), Math.sin(angle) * speed), life: 0, maxLife: 0.8 });
      this.scene.add(sprite);
    }
  }

  private stepParticles(dt: number) {
    this.particles = this.particles.filter((p) => {
      p.life += dt;
      p.velocity.y -= 12 * dt;
      p.sprite.position.addScaledVector(p.velocity, dt);
      (p.sprite.material as THREE.SpriteMaterial).opacity = Math.max(0, 1 - p.life / p.maxLife);
      if (p.life < p.maxLife) return true;
      p.sprite.removeFromParent();
      p.sprite.material.dispose();
      return false;
    });
  }

  /** Plays a frame's one-off sounds and bursts. Unknown sound names are skipped. */
  play(effects: Effect[]) {
    for (const effect of effects) {
      if (effect.kind === 'burst') this.burst(effect.x ?? 0, effect.z ?? 0, PALETTE[effect.color ?? 'gold'] ?? PALETTE.gold);
      else if (effect.name) this.sound(effect.name);
    }
  }

  private sound(name: Sound) {
    const sfx = this.sfx;
    if (!sfx) return;
    const sounds: Record<Sound, () => void> = {
      bump: () => sfx.bump(0.6),
      hit: () => sfx.hurt(),
      splash: () => sfx.splash(),
      crack: () => sfx.trap(),
      coin: () => sfx.coins(),
      win: () => sfx.win(),
      lose: () => sfx.lose(),
      tick: () => sfx.beep(0.3),
      bell: () => sfx.prize(),
      power: () => sfx.item(),
      thud: () => sfx.land(),
      pop: () => sfx.taunt(),
      blocked: () => sfx.blocked(),
      confirm: () => sfx.turn(),
    };
    sounds[name]?.();
  }

  private podium: { group: THREE.Group; rigs: { rig: CharacterRig; pose: 'cheer' | 'sad' | 'idle' }[] } | null = null;

  /** The results: the top three on a podium cheering, everyone else in front of it. Null clears it. */
  showPodium(placements: number[][] | null) {
    if (this.podium) {
      this.podium.rigs.forEach(({ rig }) => rig.dispose());
      this.podium.group.traverse((o) => {
        const mesh = o as THREE.Mesh;
        if (mesh.isMesh && mesh.userData.podium) (mesh.material as THREE.Material).dispose();
      });
      this.podium.group.removeFromParent();
      this.podium = null;
    }
    for (const drawn of this.drawn.values()) drawn.object.visible = !placements;
    if (!placements) {
      this.recamera();
      return;
    }
    const group = new THREE.Group();
    const rigs: { rig: CharacterRig; pose: 'cheer' | 'sad' | 'idle' }[] = [];
    const steps = [
      { x: 0, h: 1.3, color: '#f5c542' },
      { x: -1.7, h: 0.9, color: '#d7dde5' },
      { x: 1.7, h: 0.6, color: '#d08b4c' },
    ];
    const stand = (seat: number, x: number, y: number, z: number, pose: 'cheer' | 'sad' | 'idle') => {
      const def = this.roster.get(seat)?.def ?? characterFor(null, seat);
      const rig = buildCharacter(def);
      rig.group.position.set(x, y, z);
      group.add(rig.group);
      rigs.push({ rig, pose });
    };
    let place = 0;
    const rest: number[] = [];
    for (const tier of placements) {
      for (let i = 0; i < tier.length; i++) {
        const step = steps[place];
        if (!step) {
          rest.push(tier[i]);
          continue;
        }
        const spread = tier.length > 1 ? (i - (tier.length - 1) / 2) * 0.7 : 0;
        stand(tier[i], step.x + spread, step.h, 0, place === 0 ? 'cheer' : 'idle');
      }
      place += tier.length > 1 && place < steps.length ? 1 : tier.length;
    }
    steps.forEach((step, i) => {
      if (i >= Math.min(steps.length, placements.flat().length)) return;
      const material = new THREE.MeshStandardMaterial({ color: step.color, roughness: 0.6 });
      const block = new THREE.Mesh(this.geometry.box, material);
      block.userData.podium = true;
      block.scale.set(1.6, step.h, 1.4);
      block.position.set(step.x, step.h / 2, 0);
      block.castShadow = block.receiveShadow = true;
      group.add(block);
    });
    rest.forEach((seat, i) => stand(seat, (i - (rest.length - 1) / 2) * 1.1, 0, 2.1, 'sad'));
    const floor = new THREE.Mesh(this.geometry.disc, new THREE.MeshStandardMaterial({ color: '#f3d99a', roughness: 1 }));
    floor.userData.podium = true;
    floor.scale.set(6, 0.4, 6);
    floor.position.y = -0.2;
    floor.receiveShadow = true;
    group.add(floor);
    this.scene.add(group);
    this.podium = { group, rigs };
    this.recamera();
  }

  private drawPodium(now: number, dt: number) {
    for (const { rig, pose } of this.podium!.rigs) rig.animate(pose, now, 0);
    this.stage.update(now);
    this.stepParticles(dt);
    this.frameBox(4.6, 3.2, 18);
    this.follow.set(0, 0.9, 0);
    this.camera.position.copy(this.follow).add(this.offset);
    this.camera.lookAt(0, 1.1, 0);
    this.flow.apply(this.camera, 'podium', now);
    this.renderer.shadowMap.needsUpdate = true;
    this.renderer.render(this.scene, this.camera);
  }

  /** Draws one moment. `time` is the scene's clock, `now` the page's in seconds. */
  draw(sample: Sample | null, now: number, dt: number) {
    if (this.podium) return this.drawPodium(now, dt);
    if (!sample) {
      this.stage.update(now);
      this.renderer.render(this.scene, this.camera);
      return;
    }
    const { state, entities, speeds } = sample;
    this.setStage(state.stage, state.camera);
    const seen = new Set<string>();
    for (const entity of entities) {
      seen.add(entity.id);
      let drawn = this.drawn.get(entity.id);
      if (drawn && entity.kind === 'player') {
        const def = this.roster.get(entity.seat ?? 1)?.def;
        if (def && this.lookKey(entity.seat ?? 1, def) !== drawn.look) {
          this.release(drawn);
          drawn = undefined;
        }
      }
      if (drawn && drawn.kind !== entity.kind) {
        this.release(drawn);
        drawn = undefined;
      }
      if (!drawn) {
        drawn = this.build(entity);
        this.drawn.set(entity.id, drawn);
      }
      drawn.object.visible = !entity.hidden;
      if (entity.kind === 'player') {
        this.placePlayer(drawn, entity, entities, speeds.get(entity.id) ?? 0, now, dt);
      } else this.placeShape(drawn, entity, entities);
    }
    for (const [id, drawn] of this.drawn) {
      if (seen.has(id)) continue;
      this.release(drawn);
      this.drawn.delete(id);
    }
    this.stage.update(now);
    this.stepParticles(dt);
    this.placeCamera(state.camera, dt, now);
    const ratio = this.budget.sample(now * 1000);
    if (ratio !== null) this.renderer.setPixelRatio(ratio);
    if (now - this.lastShadow > 1 / (this.mobile ? 15 : 30)) {
      this.renderer.shadowMap.needsUpdate = true;
      this.lastShadow = now;
    }
    this.renderer.render(this.scene, this.camera);
  }

  /** Where a point on the canvas meets the play surface, for aiming with the mouse. */
  groundPoint(clientX: number, clientY: number): { x: number; z: number } | null {
    const rect = this.canvas.getBoundingClientRect();
    if (!rect.width || !rect.height) return null;
    const pointer = new THREE.Vector2(((clientX - rect.left) / rect.width) * 2 - 1, -((clientY - rect.top) / rect.height) * 2 + 1);
    this.ray.setFromCamera(pointer, this.camera);
    const hit = new THREE.Vector3();
    return this.ray.ray.intersectPlane(this.plane, hit) ? { x: hit.x, z: hit.z } : null;
  }

  /** Where a seat's character stands now, for aiming relative to it. */
  seatPosition(seat: number): { x: number; z: number } | null {
    for (const drawn of this.drawn.values()) if (drawn.kind === 'player' && drawn.seat === seat) return { x: drawn.object.position.x, z: drawn.object.position.z };
    return null;
  }

  /** Points a scene-space position at the canvas, for HUD labels such as a podium. */
  project(x: number, y: number, z: number) {
    const v = new THREE.Vector3(x, y, z).project(this.camera);
    return { x: (v.x + 1) / 2, y: (1 - v.y) / 2, visible: v.z < 1 };
  }

  /**
   * How busy the last drawn image is: the spread of brightness over a small
   * copy, how many distinct colours it has, and how many scene things are in
   * view. A blank or broken frame scores near zero on all three. Needs
   * `preserve` so the image is still there to read.
   */
  measure() {
    const small = document.createElement('canvas');
    small.width = 96;
    small.height = 54;
    const ctx = small.getContext('2d', { willReadFrequently: true })!;
    ctx.drawImage(this.canvas, 0, 0, small.width, small.height);
    const { data } = ctx.getImageData(0, 0, small.width, small.height);
    let sum = 0;
    let squares = 0;
    const colors = new Set<number>();
    const pixels = data.length / 4;
    for (let i = 0; i < data.length; i += 4) {
      const light = 0.2126 * data[i] + 0.7152 * data[i + 1] + 0.0722 * data[i + 2];
      sum += light;
      squares += light * light;
      colors.add(((data[i] >> 4) << 8) | ((data[i + 1] >> 4) << 4) | (data[i + 2] >> 4));
    }
    const mean = sum / pixels;
    const frustum = new THREE.Frustum().setFromProjectionMatrix(new THREE.Matrix4().multiplyMatrices(this.camera.projectionMatrix, this.camera.matrixWorldInverse));
    let inView = 0;
    const box = new THREE.Box3();
    const shown = this.podium ? this.podium.rigs.map(({ rig }) => rig.group) : [...this.drawn.values()].filter((d) => d.object.visible).map((d) => d.object);
    for (const object of shown) {
      box.setFromObject(object);
      if (!box.isEmpty() && frustum.intersectsBox(box)) inView++;
    }
    return { mean, stddev: Math.sqrt(Math.max(0, squares / pixels - mean * mean)), colors: colors.size, in_view: inView, drawn: this.podium ? this.podium.rigs.length : this.drawn.size };
  }

  /** Resolves once every character's authored parts (such as a pumpkin head) have loaded, or after `ms`. */
  async settle(ms = 5000) {
    const rigs = [...this.drawn.values()].flatMap((drawn) => (drawn.rig?.ready ? [drawn.rig.ready] : []));
    const podium = this.podium?.rigs.flatMap(({ rig }) => (rig.ready ? [rig.ready] : [])) ?? [];
    await Promise.race([Promise.allSettled([...rigs, ...podium]), new Promise((resolve) => setTimeout(resolve, ms))]);
  }

  /** Moves the camera straight to its shot; `hard` skips the eased handoff too, for stills. */
  recamera(hard = false) {
    this.snapCamera = true;
    if (hard) this.flow.reset();
    else this.flow.transition();
  }

  dispose() {
    this.showPodium(null);
    for (const drawn of this.drawn.values()) this.release(drawn);
    this.drawn.clear();
    for (const p of this.particles) p.sprite.material.dispose();
    this.particles = [];
    this.drop.dispose();
    this.ballTexture.dispose();
    this.iceTexture.dispose();
    Object.values(this.geometry).forEach((g) => g.dispose());
    this.groundMaterial.dispose();
    this.stage.dispose();
    this.sfx?.dispose();
    this.renderer.dispose();
  }
}
