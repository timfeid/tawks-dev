import * as THREE from 'three';
import { buildCharacter, type CharacterRig } from '../characters';
import { MAX_SPEED, stepLocal, type Court, type LocalInput, type LocalState } from '../fpmove';
import type { CharacterDef, DodgeballPlayer, DodgeballSetup, DodgeballSnap, GameEvent, Member } from '../protocol';

const SNAP_ERROR = 2.5;
const NUDGE_ERROR = 0.8;
const BALL_COLOR = '#ff5f6d';

type PlayerView = {
  seat: number;
  characterId: string;
  rig: CharacterRig;
  tag: THREE.Sprite;
  position: THREE.Vector3;
  yaw: number;
};

type BallView = { id: number; mesh: THREE.Mesh; live: boolean };
type Puff = { sprite: THREE.Sprite; life: number; rise: number };

function nameTexture(text: string, color: string) {
  const canvas = document.createElement('canvas');
  const ctx = canvas.getContext('2d')!;
  ctx.font = '600 40px system-ui, sans-serif';
  canvas.width = Math.ceil(ctx.measureText(text).width) + 40;
  canvas.height = 64;
  ctx.font = '600 40px system-ui, sans-serif';
  ctx.textBaseline = 'middle';
  ctx.lineWidth = 8;
  ctx.lineJoin = 'round';
  ctx.strokeStyle = 'rgba(15,23,42,0.85)';
  ctx.strokeText(text, 20, 34);
  ctx.fillStyle = color;
  ctx.fillText(text, 20, 34);
  const texture = new THREE.CanvasTexture(canvas);
  texture.colorSpace = THREE.SRGBColorSpace;
  return texture;
}

function floorTexture(halfX: number, halfZ: number) {
  const canvas = document.createElement('canvas');
  canvas.width = 512;
  canvas.height = 512;
  const ctx = canvas.getContext('2d')!;
  ctx.fillStyle = '#e8c88f';
  ctx.fillRect(0, 0, 512, 512);
  ctx.strokeStyle = 'rgba(150,110,60,0.25)';
  ctx.lineWidth = 2;
  for (let i = 0; i < 32; i++) {
    ctx.beginPath();
    ctx.moveTo(0, i * 16);
    ctx.lineTo(512, i * 16);
    ctx.stroke();
  }
  // The centre line and the two service arcs, painted on.
  ctx.strokeStyle = '#d6455b';
  ctx.lineWidth = 8;
  ctx.beginPath();
  ctx.moveTo(0, 256);
  ctx.lineTo(512, 256);
  ctx.stroke();
  ctx.strokeStyle = '#2f6fb8';
  ctx.lineWidth = 6;
  ctx.strokeRect(24, 24, 464, 464);
  const texture = new THREE.CanvasTexture(canvas);
  texture.colorSpace = THREE.SRGBColorSpace;
  texture.repeat.set(1, 1);
  void halfX;
  void halfZ;
  return texture;
}

function puffTexture() {
  const canvas = document.createElement('canvas');
  canvas.width = 64;
  canvas.height = 64;
  const ctx = canvas.getContext('2d')!;
  const gradient = ctx.createRadialGradient(32, 32, 4, 32, 32, 30);
  gradient.addColorStop(0, 'rgba(255,255,255,0.95)');
  gradient.addColorStop(1, 'rgba(255,255,255,0)');
  ctx.fillStyle = gradient;
  ctx.fillRect(0, 0, 64, 64);
  const texture = new THREE.CanvasTexture(canvas);
  texture.colorSpace = THREE.SRGBColorSpace;
  return texture;
}

/**
 * The dodgeball court: a gym with a painted centre line, five balls and
 * everyone in first person. Your own running is simulated here and corrected
 * against the server; everyone else comes from the interpolated snapshot.
 */
export class DodgeballCourt {
  readonly group = new THREE.Group();
  private setup: DodgeballSetup | null = null;
  private built = false;
  private players = new Map<number, PlayerView>();
  private balls = new Map<number, BallView>();
  private puffs: Puff[] = [];
  private geometries: THREE.BufferGeometry[] = [];
  private materials: THREE.Material[] = [];
  private textures: THREE.Texture[] = [];
  private puffMap = puffTexture();
  private mySeat: number | null = null;
  private local: LocalState | null = null;
  private view = new THREE.Group();
  private held: THREE.Mesh | null = null;
  private viewBob = 0;
  private bobPower = 0;
  private throwKick = 0;
  private roster = new Map<string, CharacterDef>();

  firstPerson = false;
  shake = 0;

  constructor(scene: THREE.Scene) {
    this.group.name = 'party-dodgeball';
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

  private build(setup: DodgeballSetup) {
    this.setup = setup;
    const { half_x: halfX, half_z: halfZ } = setup;
    const boards = floorTexture(halfX, halfZ);
    this.textures.push(boards);
    const floor = new THREE.Mesh(
      this.track(new THREE.BoxGeometry(halfX * 2, 0.4, halfZ * 2)),
      this.trackMaterial(new THREE.MeshStandardMaterial({ map: boards, roughness: 0.75 })),
    );
    floor.position.y = -0.2;
    floor.receiveShadow = true;
    this.group.add(floor);

    const wall = this.trackMaterial(new THREE.MeshStandardMaterial({ color: '#4c6b8a', roughness: 0.9 }));
    const trim = this.trackMaterial(new THREE.MeshStandardMaterial({ color: '#ffd23f', roughness: 0.6 }));
    for (const [dx, dz] of [
      [1, 0],
      [-1, 0],
      [0, 1],
      [0, -1],
    ]) {
      const panel = new THREE.Mesh(
        this.track(new THREE.BoxGeometry(dx ? 0.5 : halfX * 2 + 1, 3.5, dz ? 0.5 : halfZ * 2 + 1)),
        wall,
      );
      panel.position.set(dx * (halfX + 0.25), 1.75, dz * (halfZ + 0.25));
      panel.receiveShadow = true;
      this.group.add(panel);
      const rail = new THREE.Mesh(
        this.track(new THREE.BoxGeometry(dx ? 0.6 : halfX * 2 + 1.1, 0.2, dz ? 0.6 : halfZ * 2 + 1.1)),
        trim,
      );
      rail.position.set(dx * (halfX + 0.25), 3.55, dz * (halfZ + 0.25));
      this.group.add(rail);
    }

    const crate = this.trackMaterial(new THREE.MeshStandardMaterial({ color: '#f0e2c0', roughness: 0.8 }));
    const crateTop = this.trackMaterial(new THREE.MeshStandardMaterial({ color: '#6fb7d8', roughness: 0.7 }));
    for (const block of setup.blocks) {
      const mesh = new THREE.Mesh(this.track(new THREE.BoxGeometry(block.hx * 2, block.top, block.hz * 2)), crate);
      mesh.position.set(block.x, block.top / 2, block.z);
      mesh.castShadow = true;
      mesh.receiveShadow = true;
      this.group.add(mesh);
      const cap = new THREE.Mesh(this.track(new THREE.BoxGeometry(block.hx * 2 + 0.1, 0.1, block.hz * 2 + 0.1)), crateTop);
      cap.position.set(block.x, block.top + 0.05, block.z);
      this.group.add(cap);
    }
    this.built = true;
  }

  private ballMesh() {
    const setup = this.setup!;
    const mesh = new THREE.Mesh(
      this.track(new THREE.SphereGeometry(setup.ball_radius, 18, 14)),
      this.trackMaterial(new THREE.MeshStandardMaterial({ color: BALL_COLOR, roughness: 0.55 })),
    );
    mesh.castShadow = true;
    return mesh;
  }

  /** The ball in your hands, drawn in front of the camera. */
  viewModel(camera: THREE.Camera) {
    if (!this.setup) return;
    if (this.view.children.length === 0) {
      this.held = this.ballMesh();
      this.held.position.set(0.32, -0.3, -0.85);
      this.view.add(this.held);
      camera.add(this.view);
    }
    this.view.visible = this.firstPerson;
  }

  sync(
    snap: DodgeballSnap | null,
    setup: DodgeballSetup | null,
    members: Member[],
    roster: Map<string, CharacterDef>,
    mySeat: number | null,
  ) {
    this.roster = roster;
    this.mySeat = mySeat;
    if (setup && !this.built) this.build(setup);
    if (!snap || !this.built) return;

    for (const [seat, player] of this.players) {
      const member = members.find((m) => m.seat === seat);
      if (!snap.players.some((p) => p.seat === seat) || !member || member.character !== player.characterId) {
        player.rig.dispose();
        player.tag.material.map?.dispose();
        player.tag.material.dispose();
        player.tag.removeFromParent();
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
      this.group.add(rig.group, tag);
      this.players.set(player.seat, {
        seat: player.seat,
        characterId: def.id,
        rig,
        tag,
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

    for (const [id, ball] of this.balls) {
      if (!snap.balls.some((b) => b.id === id)) {
        ball.mesh.removeFromParent();
        this.balls.delete(id);
      }
    }
    for (const ball of snap.balls) {
      let view = this.balls.get(ball.id);
      if (!view) {
        view = { id: ball.id, mesh: this.ballMesh(), live: false };
        this.group.add(view.mesh);
        this.balls.set(ball.id, view);
      }
      const mine = this.firstPerson && ball.state === 'held' && ball.by === mySeat;
      view.mesh.visible = !mine;
      view.mesh.position.set(ball.x, ball.y, ball.z);
      const material = view.mesh.material as THREE.MeshStandardMaterial;
      const live = ball.state === 'live';
      if (live !== view.live) {
        view.live = live;
        material.color.set(live ? this.colorOf(ball.by ?? -1, [], BALL_COLOR) : BALL_COLOR);
        material.emissive.set(live ? '#ff9db0' : '#000000');
        material.emissiveIntensity = live ? 0.45 : 0;
      }
    }
  }

  private reconcile(me: DodgeballPlayer) {
    if (!this.local || me.out) {
      this.local = { x: me.x, y: me.y, z: me.z, vx: me.vx, vy: 0, vz: me.vz, onGround: me.ground };
      return;
    }
    const error = Math.hypot(this.local.x - me.x, this.local.z - me.z);
    if (error > SNAP_ERROR) {
      this.local.x = me.x;
      this.local.z = me.z;
      this.local.y = me.y;
      this.local.vx = me.vx;
      this.local.vz = me.vz;
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

  private colorOf(seat: number, members: Member[], fallback = '#ffffff') {
    const member = members.find((m) => m.seat === seat);
    return (member && this.roster.get(member.character)?.body) ?? fallback;
  }

  effect(event: GameEvent, mySeat: number | null, members: Member[]) {
    if (event.kind === 'throw') {
      if (event.seat === mySeat) this.throwKick = 0.18;
    } else if (event.kind === 'catch') {
      const at = this.players.get(event.seat)?.position;
      if (at) this.puff(at.clone().setY(at.y + 1.2), '#ffd23f');
      if (event.seat === mySeat) this.shake = Math.min(0.2, this.shake + 0.1);
    } else if (event.kind === 'out') {
      const at = this.players.get(event.seat)?.position;
      if (at) {
        for (let i = 0; i < 7; i++) {
          this.puff(
            at.clone().add(new THREE.Vector3(Math.random() - 0.5, 0.6 + Math.random() * 1.2, Math.random() - 0.5)),
            this.colorOf(event.by, members),
          );
        }
      }
      if (event.seat === mySeat) this.shake = Math.min(0.35, this.shake + 0.25);
    }
  }

  private puff(at: THREE.Vector3, color: string) {
    const material = new THREE.SpriteMaterial({ map: this.puffMap, color, transparent: true, depthWrite: false });
    const sprite = new THREE.Sprite(material);
    sprite.position.copy(at);
    sprite.scale.setScalar(0.7 + Math.random() * 0.5);
    this.group.add(sprite);
    this.puffs.push({ sprite, life: 0.7, rise: 1.4 });
  }

  update(snap: DodgeballSnap | null, time: number, dt: number) {
    this.shake = Math.max(0, this.shake - dt * 1.5);
    this.throwKick = Math.max(0, this.throwKick - dt * 1.4);
    for (const [seat, view] of this.players) {
      const player = snap?.players.find((p) => p.seat === seat);
      const hidden = this.firstPerson && seat === this.mySeat;
      view.rig.group.visible = !hidden;
      view.tag.visible = !hidden && !player?.out;
      if (!player) continue;
      const at = seat === this.mySeat && this.local ? this.local : player;
      view.rig.group.position.set(at.x, player.out ? at.y - 0.1 : at.y, at.z);
      view.rig.group.rotation.y = view.yaw + Math.PI;
      const running = Math.hypot(player.vx, player.vz) > 0.6;
      view.rig.animate(player.out ? 'sad' : running ? 'ride' : 'idle', time + seat * 0.3, running ? 7 : 0);
      view.tag.position.set(at.x, at.y + 2.25, at.z);
    }

    if (this.held) {
      this.held.position.set(
        0.32 + Math.sin(this.viewBob * 2.25) * 0.006 * this.bobPower,
        -0.3 + Math.sin(this.viewBob * 4.5) * 0.007 * this.bobPower - this.throwKick * 0.4,
        -0.85 + this.throwKick * 0.5,
      );
      const holding = !!snap?.players.find((p) => p.seat === this.mySeat)?.holding;
      this.held.visible = holding && this.firstPerson;
    }

    for (let i = this.puffs.length - 1; i >= 0; i--) {
      const puff = this.puffs[i];
      puff.life -= dt;
      puff.sprite.position.y += puff.rise * dt;
      puff.sprite.material.opacity = Math.max(0, puff.life / 0.7);
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
    this.players.forEach((player) => {
      player.rig.dispose();
      player.tag.material.map?.dispose();
      player.tag.material.dispose();
    });
    this.players.clear();
    this.balls.clear();
    this.puffs.forEach((puff) => puff.sprite.material.dispose());
    this.puffs = [];
    for (const child of [...this.group.children]) this.group.remove(child);
    for (const child of [...this.view.children]) this.view.remove(child);
    this.held = null;
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
    this.players.forEach((player) => {
      player.rig.dispose();
      player.tag.material.map?.dispose();
      player.tag.material.dispose();
    });
    this.players.clear();
    this.balls.clear();
    this.puffs.forEach((puff) => puff.sprite.material.dispose());
    this.puffMap.dispose();
    this.textures.forEach((texture) => texture.dispose());
    this.view.removeFromParent();
    this.geometries.forEach((geometry) => geometry.dispose());
    this.materials.forEach((material) => material.dispose());
    this.group.removeFromParent();
  }
}
