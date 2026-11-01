import * as THREE from 'three';
import { buildCharacter, type CharacterRig } from '../characters';
import { MAX_SPEED, stepLocal, type Block, type Court, type LocalInput, type LocalState } from '../fpmove';
import type { CharacterDef, Member } from '../protocol';

export type GhostsSetup = {
  game: 'ghosts';
  half_x: number;
  half_z: number;
  eye: number;
  height: number;
  radius: number;
  range: number;
  cone: number;
  blocks: Block[];
};

export type GhostsPlayer = {
  seat: number;
  x: number;
  y: number;
  z: number;
  vx: number;
  vz: number;
  yaw: number;
  pitch: number;
  ground: boolean;
  power: number;
  points: number;
  ghosts: number;
};

export type GhostKind = 'small' | 'gold' | 'big';

export type GhostState = {
  id: number;
  kind: GhostKind;
  x: number;
  y: number;
  z: number;
  vx: number;
  vz: number;
  phase: boolean;
  drawn: number;
  pullers: number[];
};

export type GhostsSnap = { game: 'ghosts'; time: number; limit: number; players: GhostsPlayer[]; ghosts: GhostState[] };

export type GhostsEvent =
  | { kind: 'caught'; id: number; ghost: GhostKind; seats: number[]; points: number; x: number; y: number; z: number }
  | { kind: 'phase'; id: number }
  | { kind: 'spawn'; id: number; ghost: GhostKind }
  | { kind: 'escape'; id: number };

const SNAP_ERROR = 2.5;
/** Your own vacuum is drawn smaller, close to the camera. */
const VIEW_SCALE = 0.3;
const NUDGE_ERROR = 0.8;
/** Ghost sizes, matched to the server's catch radii. */
const GHOST_SCALE: Record<GhostKind, number> = { small: 0.78, gold: 0.7, big: 1.65 };

type PlayerView = {
  seat: number;
  characterId: string;
  rig: CharacterRig;
  tag: THREE.Sprite;
  vacuum: Vacuum;
  position: THREE.Vector3;
  yaw: number;
};

type GhostView = {
  id: number;
  kind: GhostKind;
  group: THREE.Group;
  body: THREE.Group;
  materials: THREE.MeshStandardMaterial[];
  fade: number;
  facing: number;
  gone: number;
};

type Puff = { sprite: THREE.Sprite; life: number; max: number; velocity: THREE.Vector3 };

function canvasTexture(width: number, height: number, draw: (ctx: CanvasRenderingContext2D) => void) {
  const canvas = document.createElement('canvas');
  canvas.width = width;
  canvas.height = height;
  draw(canvas.getContext('2d')!);
  const texture = new THREE.CanvasTexture(canvas);
  texture.colorSpace = THREE.SRGBColorSpace;
  return texture;
}

function nameTexture(text: string, color: string) {
  const measure = document.createElement('canvas').getContext('2d')!;
  measure.font = '600 40px system-ui, sans-serif';
  const width = Math.ceil(measure.measureText(text).width) + 40;
  return canvasTexture(width, 64, (ctx) => {
    ctx.font = '600 40px system-ui, sans-serif';
    ctx.textBaseline = 'middle';
    ctx.lineWidth = 8;
    ctx.lineJoin = 'round';
    ctx.strokeStyle = 'rgba(15,23,42,0.85)';
    ctx.strokeText(text, 20, 34);
    ctx.fillStyle = color;
    ctx.fillText(text, 20, 34);
  });
}

function floorTexture() {
  const texture = canvasTexture(256, 256, (ctx) => {
    ctx.fillStyle = '#3b2c3f';
    ctx.fillRect(0, 0, 256, 256);
    for (let row = 0; row < 8; row++) {
      ctx.fillStyle = row % 2 ? '#43324a' : '#36283b';
      ctx.fillRect(0, row * 32 + 1, 256, 30);
      ctx.fillStyle = 'rgba(0,0,0,0.35)';
      const seam = ((row * 97) % 200) + 28;
      ctx.fillRect(seam, row * 32, 3, 32);
    }
  });
  texture.wrapS = THREE.RepeatWrapping;
  texture.wrapT = THREE.RepeatWrapping;
  return texture;
}

function puffTexture() {
  return canvasTexture(64, 64, (ctx) => {
    const gradient = ctx.createRadialGradient(32, 32, 4, 32, 32, 30);
    gradient.addColorStop(0, 'rgba(255,255,255,0.95)');
    gradient.addColorStop(1, 'rgba(255,255,255,0)');
    ctx.fillStyle = gradient;
    ctx.fillRect(0, 0, 64, 64);
  });
}

/** Spiral stripes for the suction cone, scrolled toward the nozzle. */
function swirlTexture() {
  const texture = canvasTexture(64, 128, (ctx) => {
    ctx.clearRect(0, 0, 64, 128);
    for (let i = 0; i < 4; i++) {
      const gradient = ctx.createLinearGradient(0, i * 32, 0, i * 32 + 32);
      gradient.addColorStop(0, 'rgba(200,230,255,0)');
      gradient.addColorStop(0.5, 'rgba(200,230,255,0.9)');
      gradient.addColorStop(1, 'rgba(200,230,255,0)');
      ctx.fillStyle = gradient;
      ctx.fillRect(0, i * 32, 64, 32);
    }
  });
  texture.wrapS = THREE.RepeatWrapping;
  texture.wrapT = THREE.RepeatWrapping;
  return texture;
}

/**
 * The tawks ghost, low-poly: a round head tapering into a curled tail, square
 * sunglasses, headphones, a smile and two stubby arms. Centred on its middle,
 * facing +z.
 */
function buildGhost(kind: GhostKind) {
  const body = new THREE.Group();
  const materials: THREE.MeshStandardMaterial[] = [];
  const material = (params: THREE.MeshStandardMaterialParameters) => {
    const made = new THREE.MeshStandardMaterial({ transparent: true, flatShading: true, ...params });
    materials.push(made);
    return made;
  };
  const gold = kind === 'gold';
  const skin = material({
    color: gold ? '#ffd23f' : '#eef1f6',
    emissive: gold ? '#ffae00' : '#b9c6ff',
    emissiveIntensity: gold ? 0.55 : 0.3,
    roughness: 0.55,
    metalness: gold ? 0.35 : 0,
  });
  const dark = material({ color: '#1d1d22', roughness: 0.4 });
  const cup = material({ color: '#55565e', roughness: 0.6 });
  const shine = material({ color: '#ffffff', emissive: '#ffffff', emissiveIntensity: 0.6, opacity: 0.8 });

  const profile = [
    [0.0, 0.62],
    [0.22, 0.58],
    [0.4, 0.46],
    [0.5, 0.26],
    [0.52, 0.04],
    [0.47, -0.2],
    [0.36, -0.42],
    [0.2, -0.6],
    [0.0, -0.7],
  ].map(([x, y]) => new THREE.Vector2(x, y));
  const shell = new THREE.Mesh(new THREE.LatheGeometry(profile, 9), skin);
  body.add(shell);

  const tail = new THREE.Mesh(new THREE.ConeGeometry(0.14, 0.45, 6), skin);
  tail.position.set(0.18, -0.74, -0.05);
  tail.rotation.z = 2.3;
  body.add(tail);

  for (const side of [-1, 1]) {
    const lens = new THREE.Mesh(new THREE.BoxGeometry(0.27, 0.19, 0.06), dark);
    lens.position.set(side * 0.17, 0.2, 0.47);
    lens.rotation.y = side * 0.22;
    body.add(lens);
    const glint = new THREE.Mesh(new THREE.BoxGeometry(0.04, 0.17, 0.01), shine);
    glint.position.set(side * 0.17 + 0.03, 0.2, 0.505);
    glint.rotation.set(0, side * 0.22, -0.7);
    body.add(glint);
    const ear = new THREE.Mesh(new THREE.CylinderGeometry(0.14, 0.14, 0.1, 8), cup);
    ear.rotation.z = Math.PI / 2;
    ear.position.set(side * 0.52, 0.18, 0);
    body.add(ear);
    const arm = new THREE.Mesh(new THREE.SphereGeometry(0.12, 6, 5), skin);
    arm.scale.set(1, 0.7, 0.8);
    arm.position.set(side * 0.53, -0.18, 0.08);
    body.add(arm);
  }
  const bridge = new THREE.Mesh(new THREE.BoxGeometry(0.1, 0.04, 0.04), dark);
  bridge.position.set(0, 0.24, 0.5);
  body.add(bridge);
  const band = new THREE.Mesh(new THREE.TorusGeometry(0.52, 0.035, 4, 12, Math.PI), cup);
  band.position.set(0, 0.18, 0);
  body.add(band);
  const smile = new THREE.Mesh(new THREE.TorusGeometry(0.1, 0.026, 4, 8, Math.PI), dark);
  smile.position.set(0, -0.04, 0.5);
  smile.rotation.z = Math.PI;
  body.add(smile);

  body.scale.setScalar(GHOST_SCALE[kind]);
  return { body, materials };
}

/** A hand vacuum: a canister, a hose and a nozzle pointing -z, with its cone. */
class Vacuum {
  readonly group = new THREE.Group();
  private cone: THREE.Mesh;
  private coneMaterial: THREE.MeshBasicMaterial;
  private ring: THREE.MeshStandardMaterial;
  private parts: THREE.BufferGeometry[] = [];
  private materials: THREE.Material[] = [];
  private spin = 0;

  constructor(color: string, length: number, cone: number, swirl: THREE.Texture) {
    const shell = new THREE.MeshStandardMaterial({ color, roughness: 0.45, metalness: 0.2 });
    const metal = new THREE.MeshStandardMaterial({ color: '#c7ccd6', roughness: 0.3, metalness: 0.7 });
    this.ring = new THREE.MeshStandardMaterial({ color: '#7fd8ff', emissive: '#4fc3ff', emissiveIntensity: 0.2 });
    this.materials.push(shell, metal, this.ring);
    const add = (geometry: THREE.BufferGeometry, material: THREE.Material, at: [number, number, number], rotateX = 0) => {
      this.parts.push(geometry);
      const mesh = new THREE.Mesh(geometry, material);
      mesh.position.set(...at);
      mesh.rotation.x = rotateX;
      this.group.add(mesh);
      return mesh;
    };
    add(new THREE.CylinderGeometry(0.14, 0.14, 0.42, 10), shell, [0, 0, 0.12], Math.PI / 2);
    add(new THREE.CylinderGeometry(0.06, 0.08, 0.5, 8), metal, [0, 0, -0.32], Math.PI / 2);
    add(new THREE.CylinderGeometry(0.13, 0.07, 0.14, 10), metal, [0, 0, -0.62], Math.PI / 2);
    add(new THREE.TorusGeometry(0.12, 0.02, 6, 14), this.ring, [0, 0, -0.69]);
    add(new THREE.BoxGeometry(0.06, 0.16, 0.12), shell, [0, -0.18, 0.18]);

    const geometry = new THREE.ConeGeometry(Math.tan(cone) * length, length, 20, 1, true);
    this.parts.push(geometry);
    this.coneMaterial = new THREE.MeshBasicMaterial({
      color: '#bfe6ff',
      map: swirl,
      transparent: true,
      opacity: 0,
      depthWrite: false,
      side: THREE.DoubleSide,
      blending: THREE.AdditiveBlending,
    });
    this.materials.push(this.coneMaterial);
    this.cone = new THREE.Mesh(geometry, this.coneMaterial);
    this.cone.rotation.x = Math.PI / 2;
    this.cone.position.z = -0.7 - length / 2;
    this.cone.visible = false;
    this.group.add(this.cone);
  }

  /** `power` from 0 to 1; the cone and the ring light up as it spins. */
  update(power: number, dt: number, strength = 0.16) {
    this.spin += dt * (1 + power * 5);
    this.cone.visible = power > 0.02;
    this.coneMaterial.opacity = power * strength;
    this.coneMaterial.map!.offset.y = this.spin;
    this.ring.emissiveIntensity = 0.2 + power * 1.6;
  }

  dispose() {
    this.parts.forEach((geometry) => geometry.dispose());
    this.materials.forEach((material) => material.dispose());
    this.group.removeFromParent();
  }
}

/**
 * The haunted hall for Ghost Busting: a dim plank floor, stone walls, four
 * columns and two tombs, the ghosts, and everyone's vacuum. Your own walking
 * runs here and is corrected against the server, as in Dodgeball.
 */
export class GhostHall {
  readonly group = new THREE.Group();
  private setup: GhostsSetup | null = null;
  private built = false;
  private players = new Map<number, PlayerView>();
  private ghosts = new Map<number, GhostView>();
  private puffs: Puff[] = [];
  private geometries: THREE.BufferGeometry[] = [];
  private materials: THREE.Material[] = [];
  private textures: THREE.Texture[] = [];
  private flames: THREE.Mesh[] = [];
  private roof: THREE.Mesh | null = null;
  private puffMap = puffTexture();
  private swirl = swirlTexture();
  private mySeat: number | null = null;
  private local: LocalState | null = null;
  private view = new THREE.Group();
  private held: Vacuum | null = null;
  private viewBob = 0;
  private bobPower = 0;
  private roster = new Map<string, CharacterDef>();

  firstPerson = false;
  shake = 0;

  constructor(scene: THREE.Scene) {
    this.group.name = 'party-ghosts';
    this.group.visible = false;
    scene.add(this.group);
  }

  private track<T extends THREE.BufferGeometry>(geometry: T): T {
    this.geometries.push(geometry);
    return geometry;
  }

  private trackMaterial<T extends THREE.Material>(material: T): T {
    this.materials.push(material);
    return material;
  }

  private court(): Court {
    const setup = this.setup!;
    return { blocks: setup.blocks, radius: setup.radius, halfX: setup.half_x, halfZ: setup.half_z };
  }

  private build(setup: GhostsSetup) {
    this.setup = setup;
    const { half_x: halfX, half_z: halfZ } = setup;
    const planks = floorTexture();
    planks.repeat.set(halfX / 4, halfZ / 4);
    this.textures.push(planks);
    const floor = new THREE.Mesh(
      this.track(new THREE.BoxGeometry(halfX * 2, 0.4, halfZ * 2)),
      this.trackMaterial(new THREE.MeshStandardMaterial({ map: planks, roughness: 0.9 })),
    );
    floor.position.y = -0.2;
    floor.receiveShadow = true;
    this.group.add(floor);

    const stone = this.trackMaterial(new THREE.MeshStandardMaterial({ color: '#4a4458', roughness: 0.95, flatShading: true }));
    const trim = this.trackMaterial(new THREE.MeshStandardMaterial({ color: '#2d2836', roughness: 0.9 }));
    const wallHeight = 4.6;
    for (const [dx, dz] of [
      [1, 0],
      [-1, 0],
      [0, 1],
      [0, -1],
    ]) {
      const panel = new THREE.Mesh(this.track(new THREE.BoxGeometry(dx ? 0.6 : halfX * 2 + 1.2, wallHeight, dz ? 0.6 : halfZ * 2 + 1.2)), stone);
      panel.position.set(dx * (halfX + 0.3), wallHeight / 2, dz * (halfZ + 0.3));
      panel.receiveShadow = true;
      this.group.add(panel);
      const cap = new THREE.Mesh(this.track(new THREE.BoxGeometry(dx ? 0.8 : halfX * 2 + 1.6, 0.25, dz ? 0.8 : halfZ * 2 + 1.6)), trim);
      cap.position.set(dx * (halfX + 0.3), wallHeight + 0.12, dz * (halfZ + 0.3));
      this.group.add(cap);
      // Arched windows with a pale moonlit glow.
      const along = dx ? halfZ : halfX;
      const glass = this.trackMaterial(new THREE.MeshStandardMaterial({ color: '#8fa6d9', emissive: '#6f8fd8', emissiveIntensity: 0.6 }));
      for (const at of [-along * 0.5, along * 0.5]) {
        const window = new THREE.Mesh(this.track(new THREE.BoxGeometry(dx ? 0.1 : 1.1, 1.6, dz ? 0.1 : 1.1)), glass);
        window.position.set(dx ? dx * (halfX - 0.02) : at, 2.6, dz ? dz * (halfZ - 0.02) : at);
        this.group.add(window);
      }
    }

    // A dark roof for those inside; the overhead camera looks in without it.
    const roof = new THREE.Mesh(
      this.track(new THREE.BoxGeometry(halfX * 2 + 1.6, 0.3, halfZ * 2 + 1.6)),
      this.trackMaterial(new THREE.MeshStandardMaterial({ color: '#1e1a26', roughness: 1 })),
    );
    roof.position.y = wallHeight + 0.4;
    this.roof = roof;
    this.group.add(roof);
    const beam = this.trackMaterial(new THREE.MeshStandardMaterial({ color: '#2a2230', roughness: 1 }));
    for (let i = -2; i <= 2; i++) {
      const rafter = new THREE.Mesh(this.track(new THREE.BoxGeometry(halfX * 2, 0.3, 0.35)), beam);
      rafter.position.set(0, -0.3, (i * halfZ) / 2.5);
      roof.add(rafter);
    }

    const column = this.trackMaterial(new THREE.MeshStandardMaterial({ color: '#6b6378', roughness: 0.9, flatShading: true }));
    const tomb = this.trackMaterial(new THREE.MeshStandardMaterial({ color: '#7a7f86', roughness: 0.85, flatShading: true }));
    const wax = this.trackMaterial(new THREE.MeshStandardMaterial({ color: '#f3ead2', roughness: 0.7 }));
    const flame = this.trackMaterial(new THREE.MeshStandardMaterial({ color: '#ffcf6b', emissive: '#ffa53a', emissiveIntensity: 1.6 }));
    for (const block of setup.blocks) {
      const tall = block.top > 2;
      const mesh = new THREE.Mesh(this.track(new THREE.BoxGeometry(block.hx * 2, block.top, block.hz * 2)), tall ? column : tomb);
      mesh.position.set(block.x, block.top / 2, block.z);
      mesh.castShadow = true;
      mesh.receiveShadow = true;
      this.group.add(mesh);
      const top = new THREE.Mesh(this.track(new THREE.BoxGeometry(block.hx * 2 + 0.2, 0.16, block.hz * 2 + 0.2)), trim);
      top.position.set(block.x, block.top + 0.08, block.z);
      this.group.add(top);
      // A candle on each tomb, and a pair on each column's capital.
      const candles = tall
        ? [[block.x - block.hx * 0.5, block.z], [block.x + block.hx * 0.5, block.z]]
        : [[block.x, block.z]];
      for (const [cx, cz] of candles) {
        const stick = new THREE.Mesh(this.track(new THREE.CylinderGeometry(0.06, 0.06, 0.3, 6)), wax);
        stick.position.set(cx, block.top + 0.31, cz);
        this.group.add(stick);
        const tip = new THREE.Mesh(this.track(new THREE.ConeGeometry(0.05, 0.14, 5)), flame);
        tip.position.set(cx, block.top + 0.54, cz);
        this.group.add(tip);
        this.flames.push(tip);
      }
    }
    this.built = true;
  }

  /** Your own vacuum, drawn in front of the camera. */
  viewModel(camera: THREE.Camera, color: string) {
    if (!this.setup) return;
    if (!this.held) {
      this.held = new Vacuum(color, (this.setup.range - 0.6) / VIEW_SCALE, this.setup.cone, this.swirl);
      this.held.group.scale.setScalar(VIEW_SCALE);
      this.held.group.rotation.y = 0.06;
      this.view.add(this.held.group);
      camera.add(this.view);
    }
    this.view.visible = this.firstPerson && this.group.visible;
  }

  colorOf(seat: number, members: Member[], fallback = '#9aa4b5') {
    const member = members.find((m) => m.seat === seat);
    return (member && this.roster.get(member.character)?.body) ?? fallback;
  }

  sync(snap: GhostsSnap | null, setup: GhostsSetup | null, members: Member[], roster: Map<string, CharacterDef>, mySeat: number | null) {
    this.roster = roster;
    this.mySeat = mySeat;
    if (setup && !this.built) this.build(setup);
    if (!snap || !this.built || !this.setup) return;

    for (const [seat, player] of this.players) {
      const member = members.find((m) => m.seat === seat);
      if (!snap.players.some((p) => p.seat === seat) || !member || member.character !== player.characterId) {
        this.dropPlayer(player);
        this.players.delete(seat);
      }
    }
    for (const player of snap.players) {
      const member = members.find((m) => m.seat === player.seat);
      const def = member && roster.get(member.character);
      if (!def || this.players.has(player.seat)) continue;
      const rig = buildCharacter(def);
      rig.group.scale.setScalar(0.92);
      const texture = nameTexture(member?.name ?? 'Player', def.body);
      this.textures.push(texture);
      const tag = new THREE.Sprite(new THREE.SpriteMaterial({ map: texture, transparent: true }));
      const image = texture.image as HTMLCanvasElement;
      tag.scale.set((image.width / image.height) * 0.42, 0.42, 1);
      const vacuum = new Vacuum(def.body, this.setup.range, this.setup.cone, this.swirl);
      this.group.add(rig.group, tag, vacuum.group);
      this.players.set(player.seat, {
        seat: player.seat,
        characterId: def.id,
        rig,
        tag,
        vacuum,
        position: new THREE.Vector3(player.x, player.y, player.z),
        yaw: player.yaw,
      });
    }
    for (const player of snap.players) {
      const view = this.players.get(player.seat);
      if (!view) continue;
      view.position.set(player.x, player.y, player.z);
      view.yaw = player.yaw;
      if (player.seat === mySeat) this.reconcile(player);
    }

    for (const ghost of snap.ghosts) {
      if (this.ghosts.has(ghost.id)) continue;
      const { body, materials } = buildGhost(ghost.kind);
      const group = new THREE.Group();
      group.add(body);
      group.position.set(ghost.x, ghost.y, ghost.z);
      this.group.add(group);
      this.ghosts.set(ghost.id, { id: ghost.id, kind: ghost.kind, group, body, materials, fade: 0, facing: 0, gone: 0 });
    }
    for (const [id, view] of this.ghosts) {
      if (!snap.ghosts.some((g) => g.id === id) && view.gone === 0) view.gone = 0.001;
    }
  }

  private dropPlayer(player: PlayerView) {
    player.rig.dispose();
    player.tag.material.map?.dispose();
    player.tag.material.dispose();
    player.tag.removeFromParent();
    this.textures = this.textures.filter((texture) => texture !== player.tag.material.map);
    player.vacuum.dispose();
  }

  private dropGhost(view: GhostView) {
    view.group.removeFromParent();
    view.body.traverse((child) => {
      if (child instanceof THREE.Mesh) child.geometry.dispose();
    });
    view.materials.forEach((material) => material.dispose());
  }

  private reconcile(me: GhostsPlayer) {
    if (!this.local) {
      this.local = { x: me.x, y: me.y, z: me.z, vx: me.vx, vy: 0, vz: me.vz, onGround: me.ground };
      return;
    }
    const error = Math.hypot(this.local.x - me.x, this.local.z - me.z);
    if (error > SNAP_ERROR) {
      Object.assign(this.local, { x: me.x, y: me.y, z: me.z, vx: me.vx, vz: me.vz });
    } else if (error > NUDGE_ERROR) {
      this.local.x += (me.x - this.local.x) * 0.15;
      this.local.z += (me.z - this.local.z) * 0.15;
    }
  }

  control(input: LocalInput, dt: number) {
    if (!this.setup || !this.local) return;
    stepLocal(this.local, input, dt, this.court());
    const speed = Math.hypot(this.local.vx, this.local.vz);
    this.viewBob += speed * dt;
    const want = this.local.onGround ? Math.min(1, speed / MAX_SPEED) : 0;
    this.bobPower += (want - this.bobPower) * Math.min(1, dt * 5);
  }

  eye(yaw: number, pitch: number) {
    const setup = this.setup;
    const at = this.local ?? (this.mySeat !== null ? this.players.get(this.mySeat)?.position : null);
    if (!setup || !at) return null;
    const bob = Math.sin(this.viewBob * 4.5) * 0.011 * this.bobPower;
    return { x: at.x, y: at.y + setup.eye + bob, z: at.z, yaw, pitch };
  }

  effect(event: GhostsEvent, members: Member[]) {
    if (event.kind === 'caught') {
      const at = new THREE.Vector3(event.x, event.y, event.z);
      const color = event.ghost === 'gold' ? '#ffd23f' : '#e6ecff';
      for (let i = 0; i < (event.ghost === 'big' ? 14 : 8); i++) {
        const velocity = new THREE.Vector3(Math.random() - 0.5, Math.random() * 0.8, Math.random() - 0.5).multiplyScalar(3);
        this.puff(at, color, velocity);
      }
      for (const seat of event.seats) {
        const player = this.players.get(seat);
        if (player) this.puff(player.position.clone().setY(player.position.y + 1.4), this.colorOf(seat, members), new THREE.Vector3(0, 1.2, 0));
      }
      if (this.mySeat !== null && event.seats.includes(this.mySeat)) this.shake = Math.min(0.12, this.shake + 0.08);
      const view = this.ghosts.get(event.id);
      if (view) view.gone = 0.001;
    } else if (event.kind === 'phase' || event.kind === 'escape') {
      const view = this.ghosts.get(event.id);
      if (view) this.puff(view.group.position, '#a99bff', new THREE.Vector3(0, 0.6, 0));
    }
  }

  private puff(at: THREE.Vector3, color: string, velocity: THREE.Vector3) {
    const material = new THREE.SpriteMaterial({ map: this.puffMap, color, transparent: true, depthWrite: false });
    const sprite = new THREE.Sprite(material);
    sprite.position.copy(at);
    sprite.scale.setScalar(0.5 + Math.random() * 0.4);
    this.group.add(sprite);
    this.puffs.push({ sprite, life: 0.7, max: 0.7, velocity });
  }

  update(snap: GhostsSnap | null, time: number, dt: number) {
    this.shake = Math.max(0, this.shake - dt * 1.5);
    const setup = this.setup;
    if (!setup) return;
    for (const [seat, view] of this.players) {
      const player = snap?.players.find((p) => p.seat === seat);
      const hidden = this.firstPerson && seat === this.mySeat;
      view.rig.group.visible = !hidden;
      view.tag.visible = !hidden;
      view.vacuum.group.visible = !hidden;
      if (!player) continue;
      const at = seat === this.mySeat && this.local ? this.local : player;
      view.rig.group.position.set(at.x, at.y, at.z);
      view.rig.group.rotation.y = view.yaw + Math.PI;
      const running = Math.hypot(player.vx, player.vz) > 0.6;
      view.rig.animate(running ? 'ride' : 'idle', time + seat * 0.3, running ? 7 : 0);
      view.tag.position.set(at.x, at.y + 2.25, at.z);
      // Held at the chest, pointing where they look.
      const forward = new THREE.Vector3(-Math.sin(view.yaw), 0, -Math.cos(view.yaw));
      const right = new THREE.Vector3(Math.cos(view.yaw), 0, -Math.sin(view.yaw));
      view.vacuum.group.position.set(at.x, at.y + setup.eye - 0.35, at.z).addScaledVector(forward, 0.35).addScaledVector(right, 0.22);
      view.vacuum.group.rotation.set(player.pitch, view.yaw, 0, 'YXZ');
      view.vacuum.update(player.power, dt, 0.12);
    }

    const me = snap?.players.find((p) => p.seat === this.mySeat) ?? null;
    if (this.held) {
      const kick = (me?.power ?? 0) * Math.sin(time * 60) * 0.004;
      this.held.group.position.set(
        0.2 + Math.sin(this.viewBob * 2.25) * 0.006 * this.bobPower + kick,
        -0.17 + Math.sin(this.viewBob * 4.5) * 0.007 * this.bobPower,
        -0.85,
      );
      this.held.update(me?.power ?? 0, dt, 0.035);
      this.view.visible = this.firstPerson && this.group.visible;
    }

    for (const [id, view] of this.ghosts) {
      const ghost = snap?.ghosts.find((g) => g.id === id);
      if (view.gone > 0) {
        view.gone += dt;
        const left = Math.max(0, 1 - view.gone / 0.35);
        view.group.scale.setScalar(left);
        if (left <= 0) {
          this.dropGhost(view);
          this.ghosts.delete(id);
        }
        continue;
      }
      if (!ghost) continue;
      const bob = Math.sin(time * 2.2 + id * 1.3) * 0.08;
      view.group.position.set(ghost.x, ghost.y + bob, ghost.z);
      // Face the vacuum pulling it, otherwise the way it is drifting.
      const puller = ghost.pullers.length ? this.players.get(ghost.pullers[0]) : null;
      let want = view.facing;
      if (puller) want = Math.atan2(puller.position.x - ghost.x, puller.position.z - ghost.z);
      else if (Math.hypot(ghost.vx, ghost.vz) > 0.3) want = Math.atan2(ghost.vx, ghost.vz);
      let delta = (want - view.facing) % (Math.PI * 2);
      if (delta > Math.PI) delta -= Math.PI * 2;
      if (delta < -Math.PI) delta += Math.PI * 2;
      view.facing += delta * Math.min(1, dt * 6);
      view.group.rotation.set(0, view.facing, Math.sin(time * 1.7 + id) * 0.12);
      // Being drawn in: it shudders, stretches toward the nozzle and shrinks.
      const drawn = ghost.drawn;
      const shudder = drawn > 0 ? Math.sin(time * 40 + id) * 0.04 * (0.4 + drawn) : 0;
      const shrink = 1 - drawn * 0.45;
      view.body.position.x = shudder;
      view.group.scale.set(shrink * (1 - drawn * 0.2), shrink * (1 - drawn * 0.15), shrink * (1 + drawn * 0.9));
      const fadeTo = ghost.phase ? 0.22 : 0.94;
      view.fade += (fadeTo - view.fade) * Math.min(1, dt * 6);
      for (const material of view.materials) material.opacity = view.fade;
    }

    if (this.roof) this.roof.visible = this.firstPerson;
    for (const flame of this.flames) flame.scale.y = 1 + Math.sin(time * 9 + flame.position.x * 3) * 0.18;

    for (let i = this.puffs.length - 1; i >= 0; i--) {
      const puff = this.puffs[i];
      puff.life -= dt;
      puff.sprite.position.addScaledVector(puff.velocity, dt);
      puff.sprite.material.opacity = Math.max(0, puff.life / puff.max);
      if (puff.life <= 0) {
        puff.sprite.material.dispose();
        puff.sprite.removeFromParent();
        this.puffs.splice(i, 1);
      }
    }
  }

  set visible(visible: boolean) {
    this.group.visible = visible;
    this.view.visible = visible && this.firstPerson;
  }

  /** Throw away everything built for the last game so the next starts clean. */
  reset() {
    this.players.forEach((player) => this.dropPlayer(player));
    this.players.clear();
    this.ghosts.forEach((view) => this.dropGhost(view));
    this.ghosts.clear();
    this.puffs.forEach((puff) => puff.sprite.material.dispose());
    this.puffs = [];
    this.held?.dispose();
    this.held = null;
    for (const child of [...this.group.children]) this.group.remove(child);
    this.flames = [];
    this.roof = null;
    this.geometries.forEach((geometry) => geometry.dispose());
    this.geometries = [];
    this.materials.forEach((material) => material.dispose());
    this.materials = [];
    this.textures.forEach((texture) => texture.dispose());
    this.textures = [];
    this.setup = null;
    this.built = false;
    this.local = null;
  }

  dispose() {
    this.reset();
    this.puffMap.dispose();
    this.swirl.dispose();
    this.view.removeFromParent();
    this.group.removeFromParent();
  }
}
