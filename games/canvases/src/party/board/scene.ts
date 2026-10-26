import * as THREE from 'three';
import { buildCharacter, type CharacterRig } from '../characters';
import type { BoardLogEntry, BoardToken, BoardView, CharacterDef, Item, Member, SpaceKind } from '../protocol';

const SPACE_COLORS: Record<SpaceKind, string> = {
  start: '#ffd23f',
  coin: '#4cc9f0',
  loss: '#ff5f6d',
  item: '#9b5de5',
  event: '#62c370',
  shop: '#ffa62b',
  hazard: '#7f1d3a',
  teleport: '#22d3ee',
  prize_spot: '#f8f0d0',
};

const HOP_SECONDS = 0.32;

type TokenView = {
  seat: number;
  characterId: string;
  rig: CharacterRig;
  space: number;
  from: THREE.Vector3;
  to: THREE.Vector3;
  hop: number;
  yaw: number;
  tag: THREE.Sprite;
  tagKey: string;
  shake: number;
};

type Floater = { sprite: THREE.Sprite; life: number; rise: number };
type Effect = { object: THREE.Mesh | THREE.Line; life: number; max: number; grow: number };

const ITEM_COLORS: Record<Item, string> = {
  bat: '#f59e0b',
  bomb: '#ef4444',
  grapple: '#22d3ee',
  trap: '#a78bfa',
};

const ITEM_LABELS: Record<Item, string> = { bat: 'Bat', bomb: 'Bomb', grapple: 'Grapple', trap: 'Trap' };

function heart(ctx: CanvasRenderingContext2D, x: number, y: number, size: number) {
  const s = size / 2;
  ctx.beginPath();
  ctx.moveTo(x, y + s * 0.9);
  ctx.bezierCurveTo(x - s * 1.6, y - s * 0.2, x - s * 0.6, y - s * 1.4, x, y - s * 0.45);
  ctx.bezierCurveTo(x + s * 0.6, y - s * 1.4, x + s * 1.6, y - s * 0.2, x, y + s * 0.9);
  ctx.closePath();
}

/// Hearts on top, then one pill per item held.
function statusTexture(token: BoardToken, maxHealth: number) {
  const canvas = document.createElement('canvas');
  canvas.width = 400;
  canvas.height = 120;
  const ctx = canvas.getContext('2d')!;
  ctx.lineJoin = 'round';
  const heartSize = 34;
  const heartsWidth = maxHealth * 42;
  for (let i = 0; i < maxHealth; i++) {
    const x = (canvas.width - heartsWidth) / 2 + 21 + i * 42;
    heart(ctx, x, 30, heartSize);
    ctx.lineWidth = 6;
    ctx.strokeStyle = 'rgba(15,23,42,0.9)';
    ctx.stroke();
    ctx.fillStyle = i < token.health ? '#f43f5e' : '#475569';
    ctx.fill();
  }
  ctx.font = '700 26px system-ui, sans-serif';
  ctx.textBaseline = 'middle';
  if (token.resting > 0) {
    const text = 'Resting';
    const w = ctx.measureText(text).width + 24;
    ctx.fillStyle = 'rgba(15,23,42,0.85)';
    ctx.beginPath();
    ctx.roundRect((canvas.width - w) / 2, 66, w, 40, 20);
    ctx.fill();
    ctx.fillStyle = '#cbd5e1';
    ctx.fillText(text, (canvas.width - w) / 2 + 12, 87);
  } else if (token.items.length) {
    const pills = token.items.map((item) => ({ item, w: ctx.measureText(ITEM_LABELS[item]).width + 22 }));
    const total = pills.reduce((sum, pill) => sum + pill.w, 0) + (pills.length - 1) * 6;
    let x = (canvas.width - total) / 2;
    for (const pill of pills) {
      ctx.fillStyle = 'rgba(15,23,42,0.85)';
      ctx.beginPath();
      ctx.roundRect(x - 3, 63, pill.w + 6, 46, 23);
      ctx.fill();
      ctx.fillStyle = ITEM_COLORS[pill.item];
      ctx.beginPath();
      ctx.roundRect(x, 66, pill.w, 40, 20);
      ctx.fill();
      ctx.fillStyle = '#0f172a';
      ctx.fillText(ITEM_LABELS[pill.item], x + 11, 87);
      x += pill.w + 6;
    }
  }
  const texture = new THREE.CanvasTexture(canvas);
  texture.colorSpace = THREE.SRGBColorSpace;
  return texture;
}

function tagKey(token: BoardToken) {
  return `${token.health}|${token.resting}|${token.items.join(',')}`;
}

function labelTexture(text: string, color: string) {
  const canvas = document.createElement('canvas');
  const ctx = canvas.getContext('2d')!;
  ctx.font = '700 44px system-ui, sans-serif';
  canvas.width = Math.ceil(ctx.measureText(text).width) + 36;
  canvas.height = 64;
  ctx.font = '700 44px system-ui, sans-serif';
  ctx.textBaseline = 'middle';
  ctx.lineWidth = 8;
  ctx.lineJoin = 'round';
  ctx.strokeStyle = 'rgba(15,23,42,0.85)';
  ctx.strokeText(text, 18, 34);
  ctx.fillStyle = color;
  ctx.fillText(text, 18, 34);
  const texture = new THREE.CanvasTexture(canvas);
  texture.colorSpace = THREE.SRGBColorSpace;
  return texture;
}

/**
 * The party board: one tile per space on a floating plate, characters
 * standing on their tile, and pop-up numbers for coins, hits and traps.
 */
export class PartyBoard {
  readonly group = new THREE.Group();
  private tiles = new Map<number, THREE.Mesh>();
  private trapMarks = new Map<number, THREE.Mesh>();
  private tokens = new Map<number, TokenView>();
  private floaters: Floater[] = [];
  private effects: Effect[] = [];
  private maxHealth = 3;
  private positions = new Map<number, THREE.Vector3>();
  private geometries: THREE.BufferGeometry[] = [];
  private materials: THREE.Material[] = [];
  private built = false;
  private lastSeen: string | null = null;
  private highlight: THREE.Mesh<THREE.RingGeometry, THREE.MeshBasicMaterial>;
  private branchMarks: THREE.Sprite[] = [];
  private aimMarks = new Map<number, { ring: THREE.Mesh; label: THREE.Sprite }>();
  private aimKey = '';
  private prize: THREE.Group | null = null;
  private prizeAt = -1;
  private extent = { cx: 0, cz: 0, halfW: 12, halfD: 9 };

  constructor(scene: THREE.Scene) {
    this.group.name = 'party-board';
    scene.add(this.group);
    this.highlight = new THREE.Mesh(
      this.track(new THREE.RingGeometry(0.62, 0.8, 32)),
      this.trackMaterial(new THREE.MeshBasicMaterial({ color: '#ffffff', transparent: true, opacity: 0.9, depthWrite: false })),
    );
    this.highlight.rotation.x = -Math.PI / 2;
    this.highlight.visible = false;
    this.group.add(this.highlight);
  }

  private track<T extends THREE.BufferGeometry>(geometry: T): T {
    this.geometries.push(geometry);
    return geometry;
  }

  private trackMaterial<T extends THREE.Material>(material: T): T {
    this.materials.push(material);
    return material;
  }

  private build(view: BoardView) {
    this.measure(view);
    const { cx, cz, halfW, halfD } = this.extent;
    const plate = new THREE.Mesh(
      this.track(new THREE.CylinderGeometry(1, 0.98, 0.7, 72)),
      this.trackMaterial(new THREE.MeshStandardMaterial({ color: '#f3d99a', roughness: 0.85 })),
    );
    plate.scale.set(halfW + 3.4, 1, halfD + 3.4);
    plate.position.set(cx, -0.36, cz);
    plate.receiveShadow = true;
    this.group.add(plate);
    const grass = new THREE.Mesh(
      this.track(new THREE.CylinderGeometry(1, 1, 0.16, 72)),
      this.trackMaterial(new THREE.MeshStandardMaterial({ color: '#7bc86c', roughness: 0.95 })),
    );
    grass.scale.set(halfW + 2.5, 1, halfD + 2.5);
    grass.position.set(cx, -0.01, cz);
    grass.receiveShadow = true;
    this.group.add(grass);

    for (const space of view.spaces) {
      this.positions.set(space.id, new THREE.Vector3(space.x, 0.1, space.z));
    }
    // Paths first, so the tiles sit on top of them.
    const pathMaterial = this.trackMaterial(new THREE.MeshStandardMaterial({ color: '#e9dcb6', roughness: 1 }));
    for (const space of view.spaces) {
      const from = this.positions.get(space.id)!;
      for (const nextId of space.next) {
        const to = this.positions.get(nextId)!;
        const length = from.distanceTo(to);
        const path = new THREE.Mesh(this.track(new THREE.BoxGeometry(length, 0.08, 0.5)), pathMaterial);
        path.position.copy(from).add(to).multiplyScalar(0.5).setY(0.06);
        path.rotation.y = Math.atan2(from.z - to.z, to.x - from.x);
        path.receiveShadow = true;
        this.group.add(path);
      }
    }

    const tileGeometry = this.track(new THREE.CylinderGeometry(0.72, 0.72, 0.18, 26));
    for (const space of view.spaces) {
      const tile = new THREE.Mesh(
        tileGeometry,
        this.trackMaterial(new THREE.MeshStandardMaterial({ color: SPACE_COLORS[space.kind], roughness: 0.6 })),
      );
      tile.position.copy(this.positions.get(space.id)!).setY(0.12);
      tile.castShadow = true;
      tile.receiveShadow = true;
      this.group.add(tile);
      this.tiles.set(space.id, tile);
      this.decorate(space.kind, tile.position);
      if (space.kind === 'teleport' && space.link !== null) {
        const to = this.positions.get(space.link);
        if (to) {
          const line = new THREE.Mesh(
            this.track(new THREE.TorusGeometry(tile.position.distanceTo(to) / 2, 0.05, 6, 40, Math.PI)),
            this.trackMaterial(new THREE.MeshBasicMaterial({ color: '#22d3ee', transparent: true, opacity: 0.5 })),
          );
          line.position.copy(tile.position).add(to).multiplyScalar(0.5).setY(0.25);
          line.rotation.y = -Math.atan2(to.z - tile.position.z, to.x - tile.position.x);
          this.group.add(line);
        }
      }
    }
    this.scatter(view);
    this.prize = this.buildPrize();
    this.group.add(this.prize);
    this.built = true;
  }

  private measure(view: BoardView) {
    const xs = view.spaces.map((space) => space.x);
    const zs = view.spaces.map((space) => space.z);
    const minX = Math.min(...xs);
    const maxX = Math.max(...xs);
    const minZ = Math.min(...zs);
    const maxZ = Math.max(...zs);
    this.extent = {
      cx: (minX + maxX) / 2,
      cz: (minZ + maxZ) / 2,
      halfW: (maxX - minX) / 2 + 1.5,
      halfD: (maxZ - minZ) / 2 + 1.5,
    };
  }

  /** How much ground the board covers, so the camera knows what to frame. */
  bounds() {
    return { center: new THREE.Vector3(this.extent.cx, 0, this.extent.cz), halfW: this.extent.halfW, halfD: this.extent.halfD };
  }

  /** Trees and rocks over the empty ground, so the size of the map reads. */
  private scatter(view: BoardView) {
    const { cx, cz, halfW, halfD } = this.extent;
    const trunk = this.track(new THREE.CylinderGeometry(0.16, 0.2, 0.9, 6));
    const leaves = this.track(new THREE.ConeGeometry(0.85, 1.7, 7));
    const rock = this.track(new THREE.DodecahedronGeometry(0.5));
    const barkMaterial = this.trackMaterial(new THREE.MeshStandardMaterial({ color: '#8a5a33', roughness: 0.9 }));
    const leafMaterial = this.trackMaterial(new THREE.MeshStandardMaterial({ color: '#3f9d4f', roughness: 0.85 }));
    const rockMaterial = this.trackMaterial(new THREE.MeshStandardMaterial({ color: '#9aa5b1', roughness: 0.95 }));
    let seed = 1337;
    const random = () => {
      seed = (seed * 1664525 + 1013904223) % 4294967296;
      return seed / 4294967296;
    };
    for (let i = 0; i < 90; i++) {
      const x = cx + (random() * 2 - 1) * (halfW + 2);
      const z = cz + (random() * 2 - 1) * (halfD + 2);
      const inside = (x - cx) ** 2 / (halfW + 2) ** 2 + (z - cz) ** 2 / (halfD + 2) ** 2 < 1;
      const clear = view.spaces.every((space) => (space.x - x) ** 2 + (space.z - z) ** 2 > 3.2);
      if (!inside || !clear) continue;
      if (random() < 0.7) {
        const bark = new THREE.Mesh(trunk, barkMaterial);
        bark.position.set(x, 0.45, z);
        const top = new THREE.Mesh(leaves, leafMaterial);
        top.position.set(x, 1.65, z);
        top.castShadow = true;
        this.group.add(bark, top);
      } else {
        const stone = new THREE.Mesh(rock, rockMaterial);
        stone.position.set(x, 0.25, z);
        stone.rotation.set(random(), random() * 6, random());
        stone.scale.setScalar(0.6 + random() * 0.7);
        stone.castShadow = true;
        this.group.add(stone);
      }
    }
  }

  /** A little marker so each space reads at a glance. */
  private decorate(kind: SpaceKind, at: THREE.Vector3) {
    const add = (geometry: THREE.BufferGeometry, color: string, y: number, rotation = 0) => {
      const mesh = new THREE.Mesh(this.track(geometry), this.trackMaterial(new THREE.MeshStandardMaterial({ color, roughness: 0.5 })));
      mesh.position.copy(at).setY(y);
      mesh.rotation.y = rotation;
      mesh.castShadow = true;
      this.group.add(mesh);
      return mesh;
    };
    if (kind === 'shop') {
      add(new THREE.ConeGeometry(0.62, 0.42, 10), '#ffffff', 0.62);
      add(new THREE.CylinderGeometry(0.07, 0.07, 0.5, 8), '#8a5a33', 0.36);
    } else if (kind === 'hazard') {
      for (const angle of [0, 2.1, 4.2]) {
        add(new THREE.ConeGeometry(0.16, 0.5, 6), '#f2f2f2', 0.42).position.add(
          new THREE.Vector3(Math.cos(angle) * 0.28, 0, Math.sin(angle) * 0.28),
        );
      }
    } else if (kind === 'teleport') {
      add(new THREE.TorusGeometry(0.4, 0.08, 8, 20), '#22d3ee', 0.4).rotation.x = -Math.PI / 2;
    } else if (kind === 'prize_spot') {
      add(new THREE.TorusGeometry(0.55, 0.05, 6, 24), '#f4c15d', 0.24).rotation.x = -Math.PI / 2;
    }
  }

  /** The prize itself: a gem that hovers over whichever spot holds it. */
  private buildPrize() {
    const prize = new THREE.Group();
    const gem = new THREE.Mesh(
      this.track(new THREE.OctahedronGeometry(0.42)),
      this.trackMaterial(new THREE.MeshStandardMaterial({ color: '#ff5fa2', roughness: 0.15, metalness: 0.35, emissive: '#7a1140', emissiveIntensity: 0.45 })),
    );
    gem.castShadow = true;
    const glow = new THREE.Mesh(
      this.track(new THREE.TorusGeometry(0.62, 0.05, 8, 28)),
      this.trackMaterial(new THREE.MeshBasicMaterial({ color: '#ffd1e6', transparent: true, opacity: 0.8 })),
    );
    glow.rotation.x = -Math.PI / 2;
    glow.position.y = -0.55;
    prize.add(gem, glow);
    return prize;
  }

  sync(view: BoardView, members: Member[], roster: Map<string, CharacterDef>) {
    if (!this.built) this.build(view);

    for (const [seat, token] of this.tokens) {
      const member = members.find((m) => m.seat === seat);
      if (!view.tokens.some((t) => t.seat === seat) || !member || member.character !== token.characterId) {
        token.rig.dispose();
        this.disposeTag(token);
        this.tokens.delete(seat);
      }
    }
    for (const token of view.tokens) {
      if (this.tokens.has(token.seat)) continue;
      const member = members.find((m) => m.seat === token.seat);
      const def = member && roster.get(member.character);
      if (!def) continue;
      const rig = buildCharacter(def);
      rig.group.scale.setScalar(0.85);
      this.group.add(rig.group);
      const at = this.spot(token.space, token.seat, view);
      const tag = new THREE.Sprite(new THREE.SpriteMaterial({ transparent: true, depthTest: false }));
      tag.renderOrder = 25;
      this.group.add(tag);
      this.tokens.set(token.seat, { seat: token.seat, characterId: def.id, rig, space: token.space, from: at, to: at, hop: 1, yaw: 0, tag, tagKey: '', shake: 0 });
    }

    this.maxHealth = Math.max(this.maxHealth, ...view.tokens.map((t) => t.health));
    for (const token of view.tokens) {
      const view3d = this.tokens.get(token.seat);
      if (!view3d) continue;
      const key = tagKey(token);
      if (key === view3d.tagKey) continue;
      view3d.tagKey = key;
      view3d.tag.material.map?.dispose();
      view3d.tag.material.map = statusTexture(token, this.maxHealth);
      view3d.tag.material.needsUpdate = true;
      view3d.tag.scale.set(2.4, 0.72, 1);
    }

    // Move anyone whose space changed, one hop at a time.
    for (const token of view.tokens) {
      const view3d = this.tokens.get(token.seat);
      if (!view3d || view3d.space === token.space) continue;
      view3d.from = view3d.rig.group.position.clone();
      view3d.to = this.spot(token.space, token.seat, view);
      view3d.hop = 0;
      view3d.space = token.space;
      view3d.yaw = Math.atan2(view3d.to.x - view3d.from.x, view3d.to.z - view3d.from.z);
    }

    for (const [space, mark] of this.trapMarks) {
      if (!view.traps.includes(space)) {
        mark.removeFromParent();
        this.trapMarks.delete(space);
      }
    }
    for (const space of view.traps) {
      if (this.trapMarks.has(space)) continue;
      const mark = new THREE.Mesh(
        this.track(new THREE.TorusGeometry(0.42, 0.09, 8, 18)),
        this.trackMaterial(new THREE.MeshStandardMaterial({ color: '#2b2d42', roughness: 0.6 })),
      );
      mark.rotation.x = -Math.PI / 2;
      mark.position.copy(this.positions.get(space) ?? new THREE.Vector3()).setY(0.24);
      this.group.add(mark);
      this.trapMarks.set(space, mark);
    }

    if (this.prize && view.prize_at !== this.prizeAt) {
      this.prizeAt = view.prize_at;
      const at = this.positions.get(view.prize_at);
      if (at) this.prize.position.copy(at).setY(1.5);
    }

    this.markBranch(view.turn.step === 'branch' ? view.turn.options.map((option) => option.node) : []);

    const active = this.positions.get(view.tokens.find((t) => t.seat === view.turn_seat)?.space ?? 0);
    this.highlight.visible = !!active;
    if (active) this.highlight.position.copy(active).setY(0.23);

    this.readLog(view, members);
  }

  /** Rings and distances over rivals while an item is being aimed. */
  setAim(marks: { seat: number; spaces: number; reachable: boolean }[]) {
    const key = marks.map((mark) => `${mark.seat}:${mark.spaces}:${mark.reachable}`).join('|');
    if (key === this.aimKey) return;
    this.aimKey = key;
    for (const [seat, mark] of this.aimMarks) {
      mark.ring.removeFromParent();
      (mark.ring.geometry as THREE.BufferGeometry).dispose();
      (mark.ring.material as THREE.Material).dispose();
      mark.label.material.map?.dispose();
      mark.label.material.dispose();
      mark.label.removeFromParent();
      this.aimMarks.delete(seat);
    }
    for (const mark of marks) {
      const color = mark.reachable ? '#ffd23f' : '#64748b';
      const ring = new THREE.Mesh(
        new THREE.RingGeometry(0.66, 0.92, 28),
        new THREE.MeshBasicMaterial({ color, transparent: true, opacity: mark.reachable ? 0.95 : 0.5, depthWrite: false }),
      );
      ring.rotation.x = -Math.PI / 2;
      ring.renderOrder = 20;
      const texture = labelTexture(`${mark.spaces}`, color);
      const label = new THREE.Sprite(new THREE.SpriteMaterial({ map: texture, transparent: true, depthTest: false }));
      const image = texture.image as HTMLCanvasElement;
      label.scale.set((image.width / image.height) * 0.75, 0.75, 1);
      label.renderOrder = 32;
      this.group.add(ring, label);
      this.aimMarks.set(mark.seat, { ring, label });
    }
    this.placeAim();
  }

  private placeAim() {
    for (const [seat, mark] of this.aimMarks) {
      const at = this.tokens.get(seat)?.rig.group.position;
      if (!at) continue;
      mark.ring.position.set(at.x, 0.26, at.z);
      mark.label.position.set(at.x, at.y + 2.3, at.z);
    }
  }

  /** Letters over the tiles a junction leads to, matching the buttons. */
  private markBranch(nodes: number[]) {
    if (this.branchMarks.length === nodes.length && nodes.every((node, i) => this.branchMarks[i].userData.node === node)) {
      return;
    }
    for (const mark of this.branchMarks) {
      mark.material.map?.dispose();
      mark.material.dispose();
      mark.removeFromParent();
    }
    this.branchMarks = nodes.map((node, index) => {
      const texture = labelTexture(['A', 'B', 'C'][index] ?? '?', '#ffffff');
      const sprite = new THREE.Sprite(new THREE.SpriteMaterial({ map: texture, transparent: true, depthTest: false }));
      const image = texture.image as HTMLCanvasElement;
      sprite.scale.set((image.width / image.height) * 0.9, 0.9, 1);
      sprite.position.copy(this.positions.get(node) ?? new THREE.Vector3()).add(new THREE.Vector3(0, 1.5, 0));
      sprite.renderOrder = 31;
      sprite.userData.node = node;
      this.group.add(sprite);
      return sprite;
    });
  }

  /** Players share a tile, so fan them out around its centre. */
  private spot(space: number, seat: number, view: BoardView) {
    const base = this.positions.get(space) ?? new THREE.Vector3();
    const sharing = view.tokens.filter((t) => t.space === space).map((t) => t.seat);
    const index = Math.max(0, sharing.indexOf(seat));
    const angle = (Math.PI * 2 * index) / Math.max(1, sharing.length) + Math.PI / 4;
    const spread = sharing.length > 1 ? 0.34 : 0;
    return new THREE.Vector3(base.x + Math.cos(angle) * spread, 0.2, base.z + Math.sin(angle) * spread);
  }

  private readLog(view: BoardView, members: Member[]) {
    // The log is capped and drops old lines, so find where we left off.
    const stamped = view.log.map((entry) => JSON.stringify(entry));
    const from = this.lastSeen === null ? stamped.length : stamped.lastIndexOf(this.lastSeen) + 1;
    const fresh = view.log.slice(from === 0 && this.lastSeen !== null ? 0 : from);
    this.lastSeen = stamped[stamped.length - 1] ?? this.lastSeen;
    for (const entry of fresh) {
      this.playEffect(entry, members);
      const seat = typeof entry.seat === 'number' ? entry.seat : typeof entry.target === 'number' ? entry.target : null;
      const at = seat === null ? null : this.tokens.get(seat)?.rig.group.position;
      if (!at) continue;
      if (entry.kind === 'space' && entry.effect === 'coins') this.float(at, `+${entry.coins}`, '#ffd23f');
      else if (entry.kind === 'space' && entry.effect === 'loss') this.float(at, `-${entry.coins}`, '#ff6b6b');
      else if (entry.kind === 'space' && entry.effect === 'toll') this.float(at, `+${entry.coins}`, '#ffd23f');
      else if (entry.kind === 'space' && entry.effect === 'item') this.float(at, 'Item!', '#c39bff');
      else if (entry.kind === 'trap') this.float(at, `Trap! -${entry.coins}`, '#ff6b6b');
      else if (entry.kind === 'knockout') this.float(at, `Knocked out! -${entry.coins}`, '#ff6b6b');
      else if (entry.kind === 'item') this.float(at, `${String(entry.item).toUpperCase()}!`, '#ffffff');
      else if (entry.kind === 'minigame') this.float(at, `+${entry.coins}`, '#ffd23f');
      else if (entry.kind === 'roll') {
        const name = members.find((m) => m.seat === entry.seat)?.name ?? '';
        this.float(at, `${name ? '' : ''}${entry.roll}`, '#ffffff');
      }
    }
  }

  private seatAt(seat: unknown) {
    return typeof seat === 'number' ? this.tokens.get(seat)?.rig.group.position ?? null : null;
  }

  /// Hits, blasts, grapples and awards: what happened, drawn where it happened.
  private playEffect(entry: BoardLogEntry, _members: Member[]) {
    if (entry.kind === 'hurt') {
      const token = this.tokens.get(entry.seat);
      const at = this.seatAt(entry.seat);
      if (token) token.shake = 0.5;
      if (at) this.float(at, `-${entry.damage} heart`, '#fb7185');
    } else if (entry.kind === 'award') {
      const at = this.seatAt(entry.seat);
      if (at) this.float(at, entry.item ? `Won a ${ITEM_LABELS[entry.item as Item] ?? entry.item}!` : 'Bag full', '#c39bff');
    } else if (entry.kind === 'item' && entry.item === 'bomb') {
      const center = this.positions.get(entry.space);
      if (center) this.blast(center);
      for (const seat of (entry.caught as number[]) ?? []) {
        const at = this.seatAt(seat);
        if (at) this.float(at, `-${entry.coins}`, '#ff6b6b');
      }
    } else if (entry.kind === 'item' && entry.item === 'grapple') {
      const from = this.seatAt(entry.by);
      const to = this.seatAt(entry.target);
      if (from && to) this.beam(from, to);
      if (to) this.float(to, `-${entry.coins}`, '#ff6b6b');
      if (from) this.float(from, `+${entry.coins}`, '#ffd23f');
    }
  }

  private blast(center: THREE.Vector3) {
    const ring = new THREE.Mesh(
      new THREE.RingGeometry(0.5, 0.75, 40),
      new THREE.MeshBasicMaterial({ color: '#ff7849', transparent: true, opacity: 0.9, depthWrite: false, side: THREE.DoubleSide }),
    );
    ring.rotation.x = -Math.PI / 2;
    ring.position.copy(center).setY(0.3);
    this.group.add(ring);
    this.effects.push({ object: ring, life: 0.9, max: 0.9, grow: 7 });
  }

  private beam(from: THREE.Vector3, to: THREE.Vector3) {
    const geometry = new THREE.BufferGeometry().setFromPoints([
      from.clone().setY(from.y + 1),
      to.clone().setY(to.y + 1),
    ]);
    const line = new THREE.Line(geometry, new THREE.LineBasicMaterial({ color: '#22d3ee', transparent: true, opacity: 1 }));
    this.group.add(line);
    this.effects.push({ object: line, life: 0.9, max: 0.9, grow: 0 });
  }

  private disposeTag(token: TokenView) {
    token.tag.material.map?.dispose();
    token.tag.material.dispose();
    token.tag.removeFromParent();
  }

  private float(at: THREE.Vector3, text: string, color: string) {
    const texture = labelTexture(text, color);
    const sprite = new THREE.Sprite(new THREE.SpriteMaterial({ map: texture, transparent: true, depthTest: false }));
    const image = texture.image as HTMLCanvasElement;
    sprite.scale.set((image.width / image.height) * 0.5, 0.5, 1);
    // Several lines for one player at once stack instead of overlapping.
    const stacked = this.floaters.filter(
      (f) => f.life > 1.1 && Math.abs(f.sprite.position.x - at.x) < 0.9 && Math.abs(f.sprite.position.z - at.z) < 0.9,
    ).length;
    sprite.position.copy(at).add(new THREE.Vector3(0, 2.9 + stacked * 0.5, 0));
    sprite.renderOrder = 30;
    this.group.add(sprite);
    this.floaters.push({ sprite, life: 1.6, rise: 0.9 });
  }

  update(time: number, dt: number) {
    for (const token of this.tokens.values()) {
      if (token.hop < 1) {
        token.hop = Math.min(1, token.hop + dt / HOP_SECONDS);
        const position = token.from.clone().lerp(token.to, token.hop);
        position.y = 0.2 + Math.sin(token.hop * Math.PI) * 0.7;
        token.rig.group.position.copy(position);
        token.rig.group.rotation.y = token.yaw;
        token.rig.animate('ride', time + token.seat, 6);
      } else {
        token.rig.group.position.copy(token.to);
        token.rig.animate('idle', time + token.seat * 0.4, 0);
      }
      if (token.shake > 0) {
        token.shake = Math.max(0, token.shake - dt);
        token.rig.group.position.x += Math.sin(time * 60) * 0.12 * (token.shake / 0.5);
      }
      token.tag.position.copy(token.rig.group.position).add(new THREE.Vector3(0, 2.25, 0));
    }
    for (let i = this.effects.length - 1; i >= 0; i--) {
      const effect = this.effects[i];
      effect.life -= dt;
      const t = 1 - effect.life / effect.max;
      if (effect.grow) effect.object.scale.setScalar(1 + t * effect.grow);
      (effect.object.material as THREE.Material & { opacity: number }).opacity = Math.max(0, effect.life / effect.max);
      if (effect.life <= 0) {
        effect.object.geometry.dispose();
        (effect.object.material as THREE.Material).dispose();
        effect.object.removeFromParent();
        this.effects.splice(i, 1);
      }
    }
    if (this.prize) {
      this.prize.position.y = 1.5 + Math.sin(time * 1.8) * 0.16;
      this.prize.rotation.y = time * 0.9;
    }
    this.branchMarks.forEach((mark, index) => {
      mark.position.y = 1.5 + Math.sin(time * 3 + index) * 0.12;
    });
    this.placeAim();
    for (const mark of this.aimMarks.values()) {
      mark.ring.scale.setScalar(1 + Math.sin(time * 5) * 0.09);
    }
    this.highlight.scale.setScalar(1 + Math.sin(time * 4) * 0.08);
    this.highlight.material.opacity = 0.6 + Math.sin(time * 4) * 0.2;

    for (let i = this.floaters.length - 1; i >= 0; i--) {
      const floater = this.floaters[i];
      floater.life -= dt;
      floater.sprite.position.y += floater.rise * dt;
      floater.sprite.material.opacity = Math.min(1, floater.life);
      if (floater.life <= 0) {
        floater.sprite.material.map?.dispose();
        floater.sprite.material.dispose();
        floater.sprite.removeFromParent();
        this.floaters.splice(i, 1);
      }
    }
  }

  /** Where the camera should look while a seat takes its turn. */
  focus(seat: number | null) {
    const token = seat === null ? null : this.tokens.get(seat);
    return token ? token.rig.group.position.clone() : new THREE.Vector3(0, 0, 0);
  }

  set visible(visible: boolean) {
    this.group.visible = visible;
  }

  dispose() {
    this.markBranch([]);
    this.setAim([]);
    this.tokens.forEach((token) => {
      token.rig.dispose();
      this.disposeTag(token);
    });
    this.tokens.clear();
    this.effects.forEach((effect) => {
      effect.object.geometry.dispose();
      (effect.object.material as THREE.Material).dispose();
    });
    this.effects = [];
    this.floaters.forEach((floater) => {
      floater.sprite.material.map?.dispose();
      floater.sprite.material.dispose();
    });
    this.geometries.forEach((geometry) => geometry.dispose());
    this.materials.forEach((material) => material.dispose());
    this.group.removeFromParent();
  }
}
