import * as THREE from 'three';
import { buildCharacter, type CharacterRig } from '../characters';
import { headingOf, turnToward } from '../facing';
import { place } from '../board/models';
import type { ArenaSetup, ArenaSnap, CharacterDef, GameEvent, Member } from '../protocol';

/** Red hauls toward -x, blue toward +x; the room HUD uses the same two. */
const TEAM_COLORS = ['#f87171', '#60a5fa'];

const TILE_COLORS: Record<string, string> = {
  solid: '#f1e3c0',
  cracking: '#e9a03b',
  falling: '#c2582f',
  gone: '#000000',
};

/** The round floors, as [pad, rim]. */
const DISC_COLORS: Record<string, [string, string]> = {
  hotpotato: ['#f0c987', '#e07a3f'],
  sumo: ['#c8744a', '#f4e4c1'],
  coins: ['#3f6b5f', '#ffd23f'],
};

type Rider = {
  seat: number;
  characterId: string;
  rig: CharacterRig;
  tag: THREE.Sprite;
  props: THREE.Group;
  purse?: THREE.Sprite;
  purseAt?: number;
};
type Puff = { sprite: THREE.Sprite; life: number };
/** A coin from the board's kit, with a ring on the floor marking where it lands. */
type Coin = { slot: THREE.Group; marker: THREE.Mesh<THREE.RingGeometry, THREE.MeshBasicMaterial>; gold: boolean };
type Rope = { line: THREE.Mesh; knot: THREE.Mesh };

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
 * The floor games: Crumble Run, Hot Potato Panic, Balloon Brawl, Sumo, Ledge
 * Shuffle and Coin Scramble. They share characters, a camera and a floor, and nothing
 * else: each one brings its own props and its own rules from the server.
 */
export class PartyArena {
  readonly group = new THREE.Group();
  private setup: ArenaSetup | null = null;
  private built = '';
  private riders = new Map<number, Rider>();
  private tiles = new Map<string, THREE.Mesh<THREE.BoxGeometry, THREE.MeshStandardMaterial>>();
  private platform: THREE.Group | null = null;
  private bomb: THREE.Group | null = null;
  private coins = new Map<number, Coin>();
  private spare: Record<'gold' | 'bronze', Coin[]> = { gold: [], bronze: [] };
  private rope: Rope | null = null;
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
    if (setup.game === 'tug') {
      this.buildDocks(setup.half, setup.half_z, setup.pit, setup.brink);
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
    // Ledge Shuffle brings its own floor of falling sections.
    if (setup.game === 'ledge') return;
    // Hot Potato, Sumo and Coin Scramble all play on a round floor, which
    // Sumo then scales down as its platform shrinks.
    const radius = setup.radius;
    const [pad, rim] = DISC_COLORS[setup.game] ?? DISC_COLORS.hotpotato;
    const disc = new THREE.Mesh(
      this.track(new THREE.CylinderGeometry(radius, radius - 0.6, 0.7, 56)),
      this.material(new THREE.MeshStandardMaterial({ color: pad, roughness: 0.9 })),
    );
    disc.position.y = -0.35;
    disc.receiveShadow = true;
    const trim = new THREE.Mesh(
      this.track(new THREE.TorusGeometry(radius, 0.18, 8, 60)),
      this.material(new THREE.MeshStandardMaterial({ color: rim, roughness: 0.6 })),
    );
    trim.rotation.x = -Math.PI / 2;
    trim.position.y = 0.02;
    this.platform = new THREE.Group();
    this.platform.add(disc, trim);
    this.group.add(this.platform);
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

  /** Two docks facing each other, with the water left open between them. */
  private buildDocks(half: number, halfZ: number, pit: number, brink: number) {
    const planks = this.material(new THREE.MeshStandardMaterial({ color: '#b07f4e', roughness: 0.92 }));
    const span = half - pit;
    for (const side of [-1, 1]) {
      const team = side < 0 ? 0 : 1;
      const dock = new THREE.Mesh(this.track(new THREE.BoxGeometry(span, 0.6, halfZ * 2)), planks);
      dock.position.set(side * (pit + span / 2), -0.3, 0);
      dock.receiveShadow = true;
      this.group.add(dock);
      const edge = new THREE.Mesh(
        this.track(new THREE.BoxGeometry(0.35, 0.5, halfZ * 2)),
        this.material(new THREE.MeshStandardMaterial({ color: TEAM_COLORS[team], roughness: 0.6 })),
      );
      edge.position.set(side * pit, 0.1, 0);
      this.group.add(edge);
      // The post each side has to haul the knot back to.
      const post = new THREE.Mesh(
        this.track(new THREE.CylinderGeometry(0.16, 0.2, 2.2, 10)),
        this.material(new THREE.MeshStandardMaterial({ color: TEAM_COLORS[team], roughness: 0.5 })),
      );
      post.position.set(side * brink, 1.1, -halfZ + 0.6);
      post.castShadow = true;
      this.group.add(post);
    }
    // Long enough that both ends stay on the planks at either end of its travel.
    const line = new THREE.Mesh(
      this.track(new THREE.CylinderGeometry(0.09, 0.09, (half - brink) * 2, 8)),
      this.material(new THREE.MeshStandardMaterial({ color: '#e9d8a6', roughness: 0.95 })),
    );
    line.rotation.z = Math.PI / 2;
    line.position.y = 1.05;
    const knot = new THREE.Mesh(
      this.track(new THREE.SphereGeometry(0.26, 14, 12)),
      this.material(new THREE.MeshStandardMaterial({ color: '#ffd23f', emissive: '#c98a00', emissiveIntensity: 0.5 })),
    );
    knot.position.y = 1.05;
    this.group.add(line, knot);
    this.rope = { line, knot };
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

  /**
   * A coin, taken from the pool when one is free: the models come from the
   * board's kit, so a coin only appears once its model has loaded.
   */
  private takeCoin(gold: boolean) {
    const spare = this.spare[gold ? 'gold' : 'bronze'].pop();
    if (spare) return spare;
    const slot = new THREE.Group();
    const marker = new THREE.Mesh(
      this.track(new THREE.RingGeometry(0.5, 0.68, 18)),
      this.material(
        new THREE.MeshBasicMaterial({ color: gold ? '#ffd23f' : '#e2b887', transparent: true, opacity: 0.5, side: THREE.DoubleSide }),
      ),
    );
    marker.rotation.x = -Math.PI / 2;
    slot.add(marker);
    this.group.add(slot);
    void place(slot, gold ? 'coin-gold' : 'coin-bronze', { x: 0, z: 0, scale: gold ? 2.6 : 1.7 });
    return { slot, marker, gold };
  }

  /** What one player is carrying, over their name. */
  private purseTag(rider: Rider, coins: number) {
    if (rider.purse && rider.purseAt === coins) return rider.purse;
    const texture = nameTexture(`${coins}`, '#ffd23f');
    const image = texture.image as HTMLCanvasElement;
    if (!rider.purse) {
      rider.purse = new THREE.Sprite(new THREE.SpriteMaterial({ map: texture, transparent: true, depthTest: false }));
      rider.purse.renderOrder = 31;
      this.group.add(rider.purse);
    } else {
      rider.purse.material.map?.dispose();
      rider.purse.material.map = texture;
    }
    rider.purseAt = coins;
    rider.purse.scale.set((image.width / image.height) * 0.5, 0.5, 1);
    return rider.purse;
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

  private clearRider(rider: Rider) {
    rider.rig.dispose();
    rider.tag.material.map?.dispose();
    rider.tag.material.dispose();
    rider.tag.removeFromParent();
    this.textures = this.textures.filter((texture) => texture !== rider.tag.material.map);
    rider.props.removeFromParent();
    rider.purse?.material.map?.dispose();
    rider.purse?.material.dispose();
    rider.purse?.removeFromParent();
  }

  sync(snap: ArenaSnap | null, setup: ArenaSetup | null, members: Member[], roster: Map<string, CharacterDef>) {
    this.roster = roster;
    if (setup && this.built !== setup.game) this.build(setup);
    if (!snap || !this.setup) return;

    for (const [seat, rider] of this.riders) {
      const member = members.find((m) => m.seat === seat);
      if (!snap.players.some((p) => p.seat === seat) || !member || member.character !== rider.characterId) {
        this.clearRider(rider);
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

    const live = new Set<number>();
    for (const coin of snap.coins ?? []) {
      live.add(coin.id);
      let held = this.coins.get(coin.id);
      if (!held) {
        held = this.takeCoin(coin.value > 1);
        this.coins.set(coin.id, held);
      }
      held.slot.visible = true;
      held.slot.position.set(coin.x, coin.y, coin.z);
      held.marker.position.y = 0.04 - coin.y;
      held.marker.visible = coin.y > 0.4;
    }
    for (const [id, held] of this.coins) {
      if (live.has(id)) continue;
      held.slot.visible = false;
      this.spare[held.gold ? 'gold' : 'bronze'].push(held);
      this.coins.delete(id);
    }

    if (snap.game === 'tug' && this.rope) {
      const rope = snap.rope ?? 0;
      this.rope.line.position.x = rope;
      this.rope.knot.position.x = rope;
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
    } else if (event.kind === 'spill') {
      const at = this.riders.get(event.seat)?.rig.group.position;
      if (at && event.coins > 0) {
        for (let i = 0; i < 8; i++) {
          this.puff(at.clone().setY(1.3).add(new THREE.Vector3(Math.random() - 0.5, Math.random() * 0.8, Math.random() - 0.5).multiplyScalar(2.2)), '#ffd23f');
        }
        this.shake = 0.26;
      }
    } else if (event.kind === 'gold') {
      this.puff(new THREE.Vector3(event.x, 0.6, event.z), '#ffd23f');
    } else if (event.kind === 'splash') {
      for (let i = 0; i < 5; i++) {
        this.puff(new THREE.Vector3(event.x + (Math.random() - 0.5) * 1.2, -2.3 + Math.random() * 0.8, event.z), '#bfe9ff');
      }
    } else if (event.kind === 'drop' || event.kind === 'over') {
      const at = this.riders.get(event.seat)?.rig.group.position;
      if (at) this.puff(at.clone().setY(0.4), '#cbd5e1');
    } else if (event.kind === 'bump') {
      const dust = Math.round(2 + event.strength * 4);
      for (let i = 0; i < dust; i++) {
        this.puff(new THREE.Vector3(event.x, 0.5 + Math.random() * 0.6, event.z), '#fdf6e3');
      }
      this.shake = Math.max(this.shake, Math.min(0.3, event.strength * event.strength * 0.2));
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
    if (this.platform && snap?.radius && this.setup && 'radius' in this.setup) {
      const closed = Math.max(0.1, snap.radius / this.setup.radius);
      this.platform.scale.set(closed, 1, closed);
    }
    for (const [seat, rider] of this.riders) {
      const player = snap?.players.find((p) => p.seat === seat);
      if (!player) continue;
      const height = player.y ?? 0;
      rider.rig.group.position.set(player.x, height, player.z);
      const speed = Math.hypot(player.vx ?? 0, player.vz ?? 0);
      if (speed > 0.3) rider.rig.group.rotation.y = turnToward(rider.rig.group.rotation.y, headingOf(player.vx ?? 0, player.vz ?? 0), dt);
      const pose = player.out || player.skidding ? 'fall' : speed > 0.8 ? 'ride' : 'idle';
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
      if (snap?.game === 'ledge' && !player.out) {
        rider.rig.group.rotation.y = (player.yaw ?? 0) + Math.PI;
        rider.rig.group.scale.setScalar(player.winding ? 1.1 : 0.95);
      }
      if (snap?.game === 'tug') {
        const away = (player.team ?? 0) === 0 ? -1 : 1;
        const pull = player.pull ?? 0;
        rider.rig.group.position.x = player.x + pull * away * 0.3;
        rider.rig.group.rotation.y = -away * (Math.PI / 2);
        rider.rig.animate(player.out ? 'fall' : 'ride', time + seat * 0.3, 2.5 + pull * 6);
      }
      if (snap?.game === 'coins') {
        const purse = this.purseTag(rider, player.purse ?? 0);
        purse.position.set(player.x, height + 3.15, player.z);
        purse.visible = (player.purse ?? 0) > 0;
        rider.rig.group.scale.setScalar(player.dashing ? 1.08 : 0.95);
      }
    }

    for (const [id, coin] of this.coins) {
      coin.slot.rotation.y = time * 2.2 + id;
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
    this.riders.forEach((rider) => this.clearRider(rider));
    this.riders.clear();
    this.tiles.clear();
    this.platform = null;
    this.coins.clear();
    this.spare = { gold: [], bronze: [] };
    this.bomb = null;
    this.rope = null;
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
    this.riders.forEach((rider) => this.clearRider(rider));
    this.riders.clear();
    this.coins.clear();
    this.spare = { gold: [], bronze: [] };
    this.puffs.forEach((puff) => puff.sprite.material.dispose());
    this.puffMap.dispose();
    this.textures.forEach((texture) => texture.dispose());
    this.geometries.forEach((geometry) => geometry.dispose());
    this.materials.forEach((material) => material.dispose());
    this.group.removeFromParent();
  }
}
