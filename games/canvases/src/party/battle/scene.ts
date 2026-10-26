import * as THREE from 'three';
import { buildCharacter, type CharacterRig } from '../characters';
import type { BattleSetup, BattleSnap, CharacterDef, GameEvent, Member } from '../protocol';

type TankView = { seat: number; group: THREE.Group; turret: THREE.Group; characterId: string };
type RaiderView = { seat: number; rig: CharacterRig; tag: THREE.Sprite; characterId: string };
type Spark = { sprite: THREE.Sprite; life: number; rise: number };

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

function sparkTexture() {
  const canvas = document.createElement('canvas');
  canvas.width = 64;
  canvas.height = 64;
  const ctx = canvas.getContext('2d')!;
  const gradient = ctx.createRadialGradient(32, 32, 2, 32, 32, 31);
  gradient.addColorStop(0, 'rgba(255,255,255,0.95)');
  gradient.addColorStop(1, 'rgba(255,255,255,0)');
  ctx.fillStyle = gradient;
  ctx.fillRect(0, 0, 64, 64);
  const texture = new THREE.CanvasTexture(canvas);
  texture.colorSpace = THREE.SRGBColorSpace;
  return texture;
}

/**
 * The battle games: Tank Clash, where little tanks bounce shells off the
 * walls, and Titan Siege, where everyone piles onto one huge toadstool while
 * it is winded.
 */
export class BattleField {
  readonly group = new THREE.Group();
  private setup: BattleSetup | null = null;
  private built = '';
  private tanks = new Map<number, TankView>();
  private raiders = new Map<number, RaiderView>();
  private shells = new Map<number, THREE.Mesh>();
  private crates: THREE.Mesh<THREE.BoxGeometry, THREE.MeshStandardMaterial>[] = [];
  private titan: THREE.Group | null = null;
  private telegraph: THREE.Mesh<THREE.RingGeometry, THREE.MeshBasicMaterial> | null = null;
  private sparks: Spark[] = [];
  private geometries: THREE.BufferGeometry[] = [];
  private materials: THREE.Material[] = [];
  private textures: THREE.Texture[] = [];
  private sparkMap = sparkTexture();
  private roster = new Map<string, CharacterDef>();
  shake = 0;

  constructor(scene: THREE.Scene) {
    this.group.name = 'party-battle';
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

  private colorOf(seat: number, members: Member[], fallback = '#ffffff') {
    const member = members.find((m) => m.seat === seat);
    return (member && this.roster.get(member.character)?.body) ?? fallback;
  }

  private build(setup: BattleSetup) {
    this.setup = setup;
    this.built = setup.game;
    if (setup.game === 'tank') {
      const floor = new THREE.Mesh(
        this.track(new THREE.BoxGeometry(setup.half * 2, 0.5, setup.half * 2)),
        this.material(new THREE.MeshStandardMaterial({ color: '#b9a887', roughness: 0.95 })),
      );
      floor.position.y = -0.25;
      floor.receiveShadow = true;
      this.group.add(floor);
      const wall = this.material(new THREE.MeshStandardMaterial({ color: '#6a5b48', roughness: 0.9 }));
      for (const [dx, dz] of [
        [1, 0],
        [-1, 0],
        [0, 1],
        [0, -1],
      ]) {
        const side = new THREE.Mesh(
          this.track(new THREE.BoxGeometry(dx ? 0.8 : setup.half * 2 + 1.6, 1.6, dz ? 0.8 : setup.half * 2 + 1.6)),
          wall,
        );
        side.position.set(dx * (setup.half + 0.4), 0.8, dz * (setup.half + 0.4));
        this.group.add(side);
      }
      const solid = this.material(new THREE.MeshStandardMaterial({ color: '#8fa1b8', roughness: 0.85 }));
      const timber = this.material(new THREE.MeshStandardMaterial({ color: '#d9a566', roughness: 0.8 }));
      for (const block of setup.crates) {
        const mesh = new THREE.Mesh(
          this.track(new THREE.BoxGeometry(block.half * 2, block.breakable ? 1.5 : 2.4, block.half * 2)),
          block.breakable ? timber : solid,
        );
        mesh.position.set(block.x, block.breakable ? 0.75 : 1.2, block.z);
        mesh.castShadow = true;
        mesh.receiveShadow = true;
        this.group.add(mesh);
        this.crates.push(mesh);
      }
      return;
    }
    // Titan Siege: a round pit with a ring of standing stones.
    const floor = new THREE.Mesh(
      this.track(new THREE.CylinderGeometry(setup.radius, setup.radius - 0.8, 0.7, 56)),
      this.material(new THREE.MeshStandardMaterial({ color: '#6b7a55', roughness: 0.95 })),
    );
    floor.position.y = -0.35;
    floor.receiveShadow = true;
    this.group.add(floor);
    const stone = this.material(new THREE.MeshStandardMaterial({ color: '#9aa5b1', roughness: 0.95 }));
    for (let i = 0; i < 10; i++) {
      const angle = (Math.PI * 2 * i) / 10;
      const pillar = new THREE.Mesh(this.track(new THREE.BoxGeometry(1.1, 3.2, 1.1)), stone);
      pillar.position.set(Math.cos(angle) * (setup.radius - 0.9), 1.6, Math.sin(angle) * (setup.radius - 0.9));
      pillar.rotation.y = angle;
      pillar.castShadow = true;
      this.group.add(pillar);
    }
  }

  private buildTank(def: CharacterDef | undefined) {
    const group = new THREE.Group();
    const body = this.material(new THREE.MeshStandardMaterial({ color: def?.body ?? '#7f8c8d', roughness: 0.6 }));
    const dark = this.material(new THREE.MeshStandardMaterial({ color: '#2f3545', roughness: 0.7 }));
    const hull = new THREE.Mesh(this.track(new THREE.BoxGeometry(1.7, 0.7, 2.3)), body);
    hull.position.y = 0.55;
    hull.castShadow = true;
    for (const side of [-1, 1]) {
      const track = new THREE.Mesh(this.track(new THREE.BoxGeometry(0.45, 0.55, 2.5)), dark);
      track.position.set(side * 0.95, 0.3, 0);
      group.add(track);
    }
    const turret = new THREE.Group();
    const dome = new THREE.Mesh(this.track(new THREE.CylinderGeometry(0.62, 0.7, 0.55, 14)), body);
    dome.position.y = 1.05;
    dome.castShadow = true;
    const barrel = new THREE.Mesh(this.track(new THREE.CylinderGeometry(0.14, 0.16, 1.7, 10)), dark);
    barrel.rotation.x = Math.PI / 2;
    barrel.position.set(0, 1.05, -1.2);
    turret.add(dome, barrel);
    group.add(hull, turret);
    this.group.add(group);
    return { group, turret };
  }

  private buildTitan(setup: Extract<BattleSetup, { game: 'titan' }>) {
    const titan = new THREE.Group();
    const stalkMaterial = this.material(new THREE.MeshStandardMaterial({ color: '#efe3cf', roughness: 0.8 }));
    const capMaterial = this.material(new THREE.MeshStandardMaterial({ color: '#c0392b', roughness: 0.6 }));
    const stalk = new THREE.Mesh(this.track(new THREE.CylinderGeometry(setup.titan * 0.7, setup.titan * 0.9, 5.2, 18)), stalkMaterial);
    stalk.position.y = 2.6;
    stalk.castShadow = true;
    const cap = new THREE.Mesh(this.track(new THREE.SphereGeometry(setup.titan * 1.5, 20, 12, 0, Math.PI * 2, 0, Math.PI / 2)), capMaterial);
    cap.position.y = 5.1;
    cap.castShadow = true;
    for (let i = 0; i < 7; i++) {
      const angle = (Math.PI * 2 * i) / 7;
      const spot = new THREE.Mesh(
        this.track(new THREE.SphereGeometry(0.42, 10, 8)),
        this.material(new THREE.MeshStandardMaterial({ color: '#f6f2e7', roughness: 0.7 })),
      );
      const reach = setup.titan * (i % 2 ? 0.9 : 1.25);
      spot.position.set(Math.cos(angle) * reach, 5.5, Math.sin(angle) * reach);
      titan.add(spot);
    }
    const core = new THREE.Mesh(
      this.track(new THREE.OctahedronGeometry(0.85)),
      this.material(new THREE.MeshStandardMaterial({ color: '#ffd166', emissive: '#f97316', emissiveIntensity: 0.4 })),
    );
    core.position.set(0, 1.8, 0);
    titan.add(stalk, cap, core);
    titan.userData.core = core;
    titan.userData.cap = cap;
    this.group.add(titan);

    const telegraph = new THREE.Mesh(
      this.track(new THREE.RingGeometry(1, 1.4, 44)),
      this.material(new THREE.MeshBasicMaterial({ color: '#ff5f6d', transparent: true, opacity: 0.5, depthWrite: false, side: THREE.DoubleSide })),
    );
    telegraph.rotation.x = -Math.PI / 2;
    telegraph.position.y = 0.06;
    telegraph.visible = false;
    this.group.add(telegraph);
    this.telegraph = telegraph;
    return titan;
  }

  sync(snap: BattleSnap | null, setup: BattleSetup | null, members: Member[], roster: Map<string, CharacterDef>) {
    this.roster = roster;
    if (setup && this.built !== setup.game) this.build(setup);
    if (!snap || !this.setup) return;

    if (snap.game === 'tank') {
      for (const [seat, view] of this.tanks) {
        const member = members.find((m) => m.seat === seat);
        if (!snap.players.some((p) => p.seat === seat) || !member || member.character !== view.characterId) {
          view.group.removeFromParent();
          this.tanks.delete(seat);
        }
      }
      for (const player of snap.players) {
        const member = members.find((m) => m.seat === player.seat);
        const def = member && roster.get(member.character);
        if (!def || this.tanks.has(player.seat)) continue;
        // No name tags on tanks: they would give away where everyone is.
        const { group, turret } = this.buildTank(def);
        this.tanks.set(player.seat, { seat: player.seat, group, turret, characterId: def.id });
      }

      const live = new Set((snap.shells ?? []).map((shell) => shell.id));
      for (const [id, mesh] of this.shells) {
        if (!live.has(id)) {
          mesh.removeFromParent();
          this.shells.delete(id);
        }
      }
      for (const shell of snap.shells ?? []) {
        let mesh = this.shells.get(shell.id);
        if (!mesh) {
          mesh = new THREE.Mesh(
            this.track(new THREE.SphereGeometry(0.3, 12, 10)),
            this.material(new THREE.MeshStandardMaterial({ color: this.colorOf(shell.by, members), emissive: '#ffcf70', emissiveIntensity: 0.7 })),
          );
          this.group.add(mesh);
          this.shells.set(shell.id, mesh);
        }
        mesh.position.set(shell.x, 0.8, shell.z);
      }

      (snap.crates ?? []).forEach((block, index) => {
        const mesh = this.crates[index];
        if (mesh) mesh.visible = block.solid;
      });
      return;
    }

    // Titan Siege.
    if (!this.titan && this.setup.game === 'titan') this.titan = this.buildTitan(this.setup);
    for (const [seat, view] of this.raiders) {
      const member = members.find((m) => m.seat === seat);
      if (!snap.players.some((p) => p.seat === seat) || !member || member.character !== view.characterId) {
        view.rig.dispose();
        view.tag.material.map?.dispose();
        view.tag.material.dispose();
        view.tag.removeFromParent();
        this.raiders.delete(seat);
      }
    }
    for (const player of snap.players) {
      const member = members.find((m) => m.seat === player.seat);
      const def = member && roster.get(member.character);
      if (!def || this.raiders.has(player.seat)) continue;
      const rig = buildCharacter(def);
      rig.group.scale.setScalar(0.95);
      const texture = nameTexture(member?.name ?? 'Player', def.body);
      this.textures.push(texture);
      const tag = new THREE.Sprite(new THREE.SpriteMaterial({ map: texture, transparent: true, depthTest: false }));
      const image = texture.image as HTMLCanvasElement;
      tag.scale.set((image.width / image.height) * 0.4, 0.4, 1);
      tag.renderOrder = 30;
      this.group.add(rig.group, tag);
      this.raiders.set(player.seat, { seat: player.seat, rig, tag, characterId: def.id });
    }
  }

  effect(event: GameEvent, members: Member[]) {
    if (event.kind === 'wreck') {
      for (let i = 0; i < 14; i++) {
        this.spark(
          new THREE.Vector3(event.x, 0.7, event.z).add(
            new THREE.Vector3(Math.random() - 0.5, Math.random() * 0.8, Math.random() - 0.5).multiplyScalar(3),
          ),
          i % 2 ? '#ffb703' : '#ff5f6d',
        );
      }
      this.shake = 0.4;
    } else if (event.kind === 'splinter' || event.kind === 'ricochet') {
      this.spark(new THREE.Vector3(event.x, 0.9, event.z), '#ffe7a3');
    } else if (event.kind === 'strike_core') {
      this.spark(new THREE.Vector3(event.x, 1.8, event.z), this.colorOf(event.seat, members));
    } else if (event.kind === 'strike') {
      this.shake = 0.35;
      for (let i = 0; i < 10; i++) {
        this.spark(
          new THREE.Vector3(event.x, 0.6, event.z).add(
            new THREE.Vector3(Math.random() - 0.5, Math.random() * 0.4, Math.random() - 0.5).multiplyScalar(6),
          ),
          '#e2d6b5',
        );
      }
    } else if (event.kind === 'floored') {
      const at = this.raiders.get(event.seat)?.rig.group.position;
      if (at) this.spark(at.clone().setY(1.0), '#cbd5e1');
    }
  }

  private spark(at: THREE.Vector3, color: string) {
    const sprite = new THREE.Sprite(
      new THREE.SpriteMaterial({ map: this.sparkMap, color, transparent: true, depthWrite: false }),
    );
    sprite.position.copy(at);
    sprite.scale.setScalar(0.7 + Math.random() * 0.6);
    this.group.add(sprite);
    this.sparks.push({ sprite, life: 0.6, rise: 1.8 });
  }

  update(snap: BattleSnap | null, time: number, dt: number) {
    this.shake = Math.max(0, this.shake - dt * 1.8);

    for (const [seat, view] of this.tanks) {
      const player = snap?.players.find((p) => p.seat === seat);
      if (!player) continue;
      view.group.position.set(player.x, player.out ? -1.2 : 0, player.z);
      view.group.rotation.y = player.hull ?? 0;
      view.turret.rotation.y = (player.turret ?? 0) - (player.hull ?? 0);
      view.group.visible = true;
    }

    if (this.titan && snap?.titan) {
      const titan = snap.titan;
      this.titan.position.set(titan.x, 0, titan.z);
      this.titan.rotation.y = titan.facing;
      const core = this.titan.userData.core as THREE.Mesh<THREE.OctahedronGeometry, THREE.MeshStandardMaterial>;
      const open = titan.state === 'winded';
      core.material.emissiveIntensity = open ? 1.6 : 0.25;
      core.material.color.set(open ? '#ffe066' : '#8d6e3f');
      core.rotation.y = time * 1.4;
      core.scale.setScalar(open ? 1.25 + Math.sin(time * 6) * 0.1 : 0.9);
      const cap = this.titan.userData.cap as THREE.Mesh;
      // It rears back as it winds up, then slams down.
      cap.position.y = 5.1 + (titan.state === 'winding' ? Math.sin(time * 9) * 0.22 : 0);
      if (this.telegraph) {
        const winding = titan.state === 'winding';
        this.telegraph.visible = winding;
        if (winding) {
          const reach = titan.attack === 'sweep' ? 8 : titan.attack === 'charge' ? 4 : 9.5;
          this.telegraph.position.set(titan.x, 0.06, titan.z);
          this.telegraph.scale.setScalar(reach);
          this.telegraph.material.opacity = 0.25 + Math.abs(Math.sin(time * 7)) * 0.45;
        }
      }
    }

    for (const [seat, view] of this.raiders) {
      const player = snap?.players.find((p) => p.seat === seat);
      if (!player) continue;
      view.rig.group.position.set(player.x, 0, player.z);
      const speed = Math.hypot(player.vx ?? 0, player.vz ?? 0);
      if (speed > 0.3) view.rig.group.rotation.y = Math.atan2(player.vx ?? 0, player.vz ?? 0) + Math.PI;
      view.rig.animate(player.out ? 'fall' : speed > 0.8 ? 'ride' : 'idle', time + seat * 0.3, speed > 0.8 ? 7 : 0);
      view.tag.position.set(player.x, 2.4, player.z);
      view.tag.visible = !player.out;
    }

    for (let i = this.sparks.length - 1; i >= 0; i--) {
      const spark = this.sparks[i];
      spark.life -= dt;
      spark.sprite.position.y += spark.rise * dt;
      spark.sprite.material.opacity = Math.max(0, spark.life / 0.6);
      if (spark.life <= 0) {
        spark.sprite.material.dispose();
        spark.sprite.removeFromParent();
        this.sparks.splice(i, 1);
      }
    }
  }

  set visible(visible: boolean) {
    this.group.visible = visible;
  }

  /** Throw away everything built for the last game so the next starts clean. */
  reset() {
    this.tanks.clear();
    this.raiders.forEach((view) => {
      view.rig.dispose();
      view.tag.material.map?.dispose();
      view.tag.material.dispose();
    });
    this.raiders.clear();
    this.shells.clear();
    this.crates = [];
    this.titan = null;
    this.telegraph = null;
    this.sparks.forEach((spark) => spark.sprite.material.dispose());
    this.sparks = [];
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
    this.reset();
    this.sparkMap.dispose();
    this.group.removeFromParent();
  }
}
