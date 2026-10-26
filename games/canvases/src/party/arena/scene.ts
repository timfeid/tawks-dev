import * as THREE from 'three';
import { buildCharacter, type CharacterRig } from '../characters';
import type { ArenaSetup, ArenaSnap, CharacterDef, GameEvent, Member } from '../protocol';

const TILE_COLORS: Record<string, string> = {
  solid: '#f1e3c0',
  cracking: '#e9a03b',
  falling: '#c2582f',
  gone: '#000000',
};

type Rider = { seat: number; characterId: string; rig: CharacterRig; tag: THREE.Sprite; props: THREE.Group };
type Puff = { sprite: THREE.Sprite; life: number };

function nameTexture(text: string, color: string) {
  const canvas = document.createElement('canvas');
  const ctx = canvas.getContext('2d')!;
  ctx.font = '600 38px system-ui, sans-serif';
  canvas.width = Math.ceil(ctx.measureText(text).width) + 36;
  canvas.height = 60;
  ctx.font = '600 38px system-ui, sans-serif';
  ctx.textBaseline = 'middle';
  ctx.lineWidth = 8;
  ctx.lineJoin = 'round';
  ctx.strokeStyle = 'rgba(15,23,42,0.85)';
  ctx.strokeText(text, 18, 32);
  ctx.fillStyle = color;
  ctx.fillText(text, 18, 32);
  const texture = new THREE.CanvasTexture(canvas);
  texture.colorSpace = THREE.SRGBColorSpace;
  return texture;
}

function puffTexture() {
  const canvas = document.createElement('canvas');
  canvas.width = 64;
  canvas.height = 64;
  const ctx = canvas.getContext('2d')!;
  const gradient = ctx.createRadialGradient(32, 32, 3, 32, 32, 31);
  gradient.addColorStop(0, 'rgba(255,255,255,0.95)');
  gradient.addColorStop(1, 'rgba(255,255,255,0)');
  ctx.fillStyle = gradient;
  ctx.fillRect(0, 0, 64, 64);
  const texture = new THREE.CanvasTexture(canvas);
  texture.colorSpace = THREE.SRGBColorSpace;
  return texture;
}

/**
 * The floor games: Crumble Run, Hot Potato Panic, Balloon Brawl and Ledge
 * Shuffle. They share characters, a camera and a floor, and nothing else: each
 * one brings its own props and its own rules from the server.
 */
export class PartyArena {
  readonly group = new THREE.Group();
  private setup: ArenaSetup | null = null;
  private built = '';
  private riders = new Map<number, Rider>();
  private tiles = new Map<string, THREE.Mesh<THREE.BoxGeometry, THREE.MeshStandardMaterial>>();
  private ledges = new Map<string, THREE.Mesh<THREE.CylinderGeometry, THREE.MeshStandardMaterial>>();
  private bomb: THREE.Group | null = null;
  private puffs: Puff[] = [];
  private geometries: THREE.BufferGeometry[] = [];
  private materials: THREE.Material[] = [];
  private textures: THREE.Texture[] = [];
  private puffMap = puffTexture();
  private roster = new Map<string, CharacterDef>();
  shake = 0;

  constructor(scene: THREE.Scene) {
    this.group.name = 'party-arena';
    this.group.visible = false;
    scene.add(this.group);
  }

  private track<T extends THREE.BufferGeometry>(geometry: T): T {
    this.geometries.push(geometry);
    return geometry;
  }

  private material<T extends THREE.Material>(material: T): T {
    this.materials.push(material);
    return material;
  }

  private build(setup: ArenaSetup) {
    this.setup = setup;
    this.built = setup.game;
    if (setup.game === 'crumble') {
      const pad = new THREE.Mesh(
        this.track(new THREE.CylinderGeometry(setup.radius + 2, setup.radius + 1, 0.8, 48)),
        this.material(new THREE.MeshStandardMaterial({ color: '#2b3240', roughness: 0.95 })),
      );
      pad.position.y = -1.4;
      this.group.add(pad);
      return;
    }
    if (setup.game === 'balloon') {
      const floor = new THREE.Mesh(
        this.track(new THREE.BoxGeometry(setup.half * 2, 0.5, setup.half * 2)),
        this.material(new THREE.MeshStandardMaterial({ color: '#8fd3c7', roughness: 0.9 })),
      );
      floor.position.y = -0.25;
      floor.receiveShadow = true;
      this.group.add(floor);
      const rail = this.material(new THREE.MeshStandardMaterial({ color: '#ffd23f', roughness: 0.6 }));
      for (const [dx, dz] of [
        [1, 0],
        [-1, 0],
        [0, 1],
        [0, -1],
      ]) {
        const wall = new THREE.Mesh(
          this.track(new THREE.BoxGeometry(dx ? 0.5 : setup.half * 2 + 1, 0.9, dz ? 0.5 : setup.half * 2 + 1)),
          rail,
        );
        wall.position.set(dx * (setup.half + 0.25), 0.45, dz * (setup.half + 0.25));
        this.group.add(wall);
      }
      return;
    }
    // Hot Potato and Ledge Shuffle both play on a round floor.
    const radius = setup.radius;
    const disc = new THREE.Mesh(
      this.track(new THREE.CylinderGeometry(radius, radius - 0.6, 0.7, 56)),
      this.material(new THREE.MeshStandardMaterial({ color: setup.game === 'ledge' ? '#3d4a63' : '#f0c987', roughness: 0.9 })),
    );
    disc.position.y = -0.35;
    disc.receiveShadow = true;
    this.group.add(disc);
    const trim = new THREE.Mesh(
      this.track(new THREE.TorusGeometry(radius, 0.18, 8, 60)),
      this.material(new THREE.MeshStandardMaterial({ color: setup.game === 'ledge' ? '#f97362' : '#e07a3f', roughness: 0.6 })),
    );
    trim.rotation.x = -Math.PI / 2;
    trim.position.y = 0.02;
    this.group.add(trim);
  }

  private tileMesh(key: string) {
    let mesh = this.tiles.get(key);
    if (!mesh) {
      const tile = this.setup && this.setup.game === 'crumble' ? this.setup.tile : 2.6;
      mesh = new THREE.Mesh(
        this.track(new THREE.BoxGeometry(tile * 0.94, 0.5, tile * 0.94)),
        this.material(new THREE.MeshStandardMaterial({ color: TILE_COLORS.solid, roughness: 0.85 })),
      );
      mesh.receiveShadow = true;
      this.group.add(mesh);
      this.tiles.set(key, mesh);
    }
    return mesh;
  }

  private ledgeMesh(key: string) {
    let mesh = this.ledges.get(key);
    if (!mesh) {
      mesh = new THREE.Mesh(
        this.track(new THREE.CylinderGeometry(1.35, 1.5, 1.1, 20)),
        this.material(new THREE.MeshStandardMaterial({ color: '#f1e3c0', roughness: 0.8 })),
      );
      mesh.castShadow = true;
      mesh.receiveShadow = true;
      this.group.add(mesh);
      this.ledges.set(key, mesh);
    }
    return mesh;
  }

  private buildBomb() {
    const bomb = new THREE.Group();
    const shell = new THREE.Mesh(
      this.track(new THREE.SphereGeometry(0.42, 16, 14)),
      this.material(new THREE.MeshStandardMaterial({ color: '#23252f', roughness: 0.45 })),
    );
    const fuse = new THREE.Mesh(
      this.track(new THREE.CylinderGeometry(0.05, 0.05, 0.35, 8)),
      this.material(new THREE.MeshStandardMaterial({ color: '#8a5a33', roughness: 0.9 })),
    );
    fuse.position.y = 0.5;
    const spark = new THREE.Mesh(
      this.track(new THREE.SphereGeometry(0.13, 10, 8)),
      this.material(new THREE.MeshStandardMaterial({ color: '#ffbf47', emissive: '#ff7a1c', emissiveIntensity: 1.4 })),
    );
    spark.position.y = 0.7;
    bomb.add(shell, fuse, spark);
    bomb.userData.spark = spark;
    this.group.add(bomb);
    return bomb;
  }

  /** Balloons on a string, one per life left. */
  private balloonProps(rider: Rider, count: number, def: CharacterDef | undefined) {
    const have = rider.props.children.length;
    if (have === count) return;
    while (rider.props.children.length > count) {
      const last = rider.props.children[rider.props.children.length - 1];
      rider.props.remove(last);
    }
    while (rider.props.children.length < count) {
      const i = rider.props.children.length;
      const balloon = new THREE.Mesh(
        this.track(new THREE.SphereGeometry(0.34, 14, 12)),
        this.material(new THREE.MeshStandardMaterial({ color: def?.accent ?? '#ff5f6d', roughness: 0.3 })),
      );
      balloon.scale.y = 1.25;
      balloon.position.set(Math.cos(i * 2.1) * 0.42, 2.1 + (i % 2) * 0.28, Math.sin(i * 2.1) * 0.42);
      rider.props.add(balloon);
    }
  }

  sync(snap: ArenaSnap | null, setup: ArenaSetup | null, members: Member[], roster: Map<string, CharacterDef>) {
    this.roster = roster;
    if (setup && this.built !== setup.game) this.build(setup);
    if (!snap || !this.setup) return;

    for (const [seat, rider] of this.riders) {
      const member = members.find((m) => m.seat === seat);
      if (!snap.players.some((p) => p.seat === seat) || !member || member.character !== rider.characterId) {
        rider.rig.dispose();
        rider.tag.material.map?.dispose();
        rider.tag.material.dispose();
        rider.tag.removeFromParent();
        rider.props.removeFromParent();
        this.riders.delete(seat);
      }
    }
    for (const player of snap.players) {
      const member = members.find((m) => m.seat === player.seat);
      const def = member && roster.get(member.character);
      if (!def || this.riders.has(player.seat)) continue;
      const rig = buildCharacter(def);
      rig.group.scale.setScalar(0.95);
      const texture = nameTexture(member?.name ?? 'Player', def.body);
      this.textures.push(texture);
      const tag = new THREE.Sprite(new THREE.SpriteMaterial({ map: texture, transparent: true, depthTest: false }));
      const image = texture.image as HTMLCanvasElement;
      tag.scale.set((image.width / image.height) * 0.4, 0.4, 1);
      tag.renderOrder = 30;
      const props = new THREE.Group();
      rig.group.add(props);
      this.group.add(rig.group, tag);
      this.riders.set(player.seat, { seat: player.seat, characterId: def.id, rig, tag, props });
    }

    for (const tile of snap.tiles ?? []) {
      const mesh = this.tileMesh(`${tile.x},${tile.z}`);
      mesh.position.set(tile.x, tile.state === 'falling' ? -0.8 : -0.25, tile.z);
      mesh.visible = tile.state !== 'gone';
      mesh.material.color.set(TILE_COLORS[tile.state] ?? TILE_COLORS.solid);
      mesh.rotation.z = tile.state === 'falling' ? 0.18 : 0;
    }

    for (const [index, ledge] of (snap.ledges ?? []).entries()) {
      const mesh = this.ledgeMesh(`${index}`);
      mesh.position.set(ledge.x, -0.05, ledge.z);
      mesh.visible = true;
      mesh.material.color.set(ledge.taken === null ? '#f1e3c0' : '#8ef5a4');
    }
    for (const [key, mesh] of this.ledges) {
      if (Number(key) >= (snap.ledges?.length ?? 0)) mesh.visible = false;
    }

    if (snap.game === 'hotpotato') {
      if (!this.bomb) this.bomb = this.buildBomb();
      const holder = snap.players.find((p) => p.seat === snap.holder);
      this.bomb.visible = !!holder;
      if (holder) this.bomb.position.set(holder.x, 2.35, holder.z);
    } else if (this.bomb) {
      this.bomb.visible = false;
    }
  }

  effect(event: GameEvent, members: Member[]) {
    const colorOf = (seat: number) => {
      const member = members.find((m) => m.seat === seat);
      return (member && this.roster.get(member.character)?.body) ?? '#ffffff';
    };
    if (event.kind === 'boom') {
      for (let i = 0; i < 18; i++) {
        this.puff(
          new THREE.Vector3(event.x, 0.8, event.z).add(
            new THREE.Vector3(Math.random() - 0.5, Math.random(), Math.random() - 0.5).multiplyScalar(4),
          ),
          i % 2 ? '#ff9f1c' : '#ff3b52',
        );
      }
      this.shake = 0.5;
    } else if (event.kind === 'pop' || event.kind === 'burst') {
      const at = this.riders.get(event.seat)?.rig.group.position;
      if (at) {
        for (let i = 0; i < 6; i++) {
          this.puff(at.clone().setY(2.2).add(new THREE.Vector3(Math.random() - 0.5, Math.random() * 0.6, Math.random() - 0.5)), colorOf(event.seat));
        }
      }
    } else if (event.kind === 'drop' || event.kind === 'over') {
      const at = this.riders.get(event.seat)?.rig.group.position;
      if (at) this.puff(at.clone().setY(0.4), '#cbd5e1');
    }
  }

  private puff(at: THREE.Vector3, color: string) {
    const sprite = new THREE.Sprite(
      new THREE.SpriteMaterial({ map: this.puffMap, color, transparent: true, depthWrite: false }),
    );
    sprite.position.copy(at);
    sprite.scale.setScalar(0.8 + Math.random() * 0.7);
    this.group.add(sprite);
    this.puffs.push({ sprite, life: 0.8 });
  }

  update(snap: ArenaSnap | null, time: number, dt: number) {
    this.shake = Math.max(0, this.shake - dt * 1.6);
    for (const [seat, rider] of this.riders) {
      const player = snap?.players.find((p) => p.seat === seat);
      if (!player) continue;
      const height = player.y ?? 0;
      rider.rig.group.position.set(player.x, height, player.z);
      const speed = Math.hypot(player.vx ?? 0, player.vz ?? 0);
      if (speed > 0.3) rider.rig.group.rotation.y = Math.atan2(player.vx ?? 0, player.vz ?? 0) + Math.PI;
      const pose = player.out ? 'fall' : speed > 0.8 ? 'ride' : 'idle';
      rider.rig.animate(pose, time + seat * 0.3, speed > 0.8 ? 7 : 0);
      rider.tag.position.set(player.x, height + 2.5, player.z);
      rider.tag.visible = !player.out;
      if (snap?.game === 'balloon') {
        const member = [...this.roster.values()].find((def) => def.id === rider.characterId);
        this.balloonProps(rider, player.balloons ?? 0, member);
        rider.props.rotation.y = time * 0.8;
        rider.rig.group.scale.setScalar(player.dashing ? 1.08 : 0.95);
      }
      if (snap?.game === 'hotpotato') {
        rider.rig.group.scale.setScalar(player.holding ? 1.06 : 0.95);
      }
    }

    if (this.bomb?.visible) {
      const spark = this.bomb.userData.spark as THREE.Mesh<THREE.SphereGeometry, THREE.MeshStandardMaterial>;
      const fuse = snap?.game === 'hotpotato' ? (snap.fuse ?? 5) : 5;
      const urgency = Math.max(0.6, 8 - fuse);
      spark.material.emissiveIntensity = 0.6 + Math.abs(Math.sin(time * urgency)) * 1.8;
      this.bomb.rotation.y = time * 1.6;
      this.bomb.position.y = 2.35 + Math.sin(time * 4) * 0.08;
    }

    for (let i = this.puffs.length - 1; i >= 0; i--) {
      const puff = this.puffs[i];
      puff.life -= dt;
      puff.sprite.position.y += dt * 1.6;
      puff.sprite.material.opacity = Math.max(0, puff.life);
      if (puff.life <= 0) {
        puff.sprite.material.dispose();
        puff.sprite.removeFromParent();
        this.puffs.splice(i, 1);
      }
    }
  }

  set visible(visible: boolean) {
    this.group.visible = visible;
  }

  /** Throw away everything built for the last game so the next starts clean. */
  reset() {
    this.riders.forEach((rider) => {
      rider.rig.dispose();
      rider.tag.material.map?.dispose();
      rider.tag.material.dispose();
    });
    this.riders.clear();
    this.tiles.clear();
    this.ledges.clear();
    this.bomb = null;
    this.puffs.forEach((puff) => puff.sprite.material.dispose());
    this.puffs = [];
    for (const child of [...this.group.children]) this.group.remove(child);
    this.geometries.forEach((geometry) => geometry.dispose());
    this.geometries = [];
    this.materials.forEach((material) => material.dispose());
    this.materials = [];
    this.textures.forEach((texture) => texture.dispose());
    this.textures = [];
    this.setup = null;
    this.built = '';
  }

  dispose() {
    this.riders.forEach((rider) => {
      rider.rig.dispose();
      rider.tag.material.map?.dispose();
      rider.tag.material.dispose();
    });
    this.riders.clear();
    this.puffs.forEach((puff) => puff.sprite.material.dispose());
    this.puffMap.dispose();
    this.textures.forEach((texture) => texture.dispose());
    this.geometries.forEach((geometry) => geometry.dispose());
    this.materials.forEach((material) => material.dispose());
    this.group.removeFromParent();
  }
}
