import * as THREE from 'three';
import { buildCharacter, type CharacterRig, type Pose } from '../characters';
import type { CharacterDef } from '../protocol';

export const TABLE_RADIUS = 6.4;
export const TABLE_TOP = 1.0;
export const SEAT_RADIUS = 8.4;
export const CARD_W = 0.9;
export const CARD_H = 1.3;

const textureCache = new Map<string, THREE.CanvasTexture>();

/** A card face: a title and a few lines of text on a coloured ground. */
export function textTexture(
  lines: string[],
  color: string,
  background: string,
  width = 256,
  height = 360,
) {
  const key = `${lines.join('|')}:${color}:${background}:${width}x${height}`;
  const cached = textureCache.get(key);
  if (cached) return cached;
  const canvas = document.createElement('canvas');
  canvas.width = width;
  canvas.height = height;
  const ctx = canvas.getContext('2d')!;
  ctx.fillStyle = background;
  ctx.fillRect(0, 0, width, height);
  ctx.strokeStyle = 'rgba(255,255,255,0.35)';
  ctx.lineWidth = 8;
  ctx.strokeRect(10, 10, width - 20, height - 20);
  ctx.fillStyle = color;
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  const size = lines.length > 1 ? 34 : 40;
  ctx.font = `800 ${size}px system-ui, sans-serif`;
  lines.forEach((line, i) =>
    ctx.fillText(line, width / 2, height / 2 + (i - (lines.length - 1) / 2) * (size + 8)),
  );
  const texture = new THREE.CanvasTexture(canvas);
  texture.colorSpace = THREE.SRGBColorSpace;
  texture.anisotropy = 4;
  textureCache.set(key, texture);
  return texture;
}

/** A floating label with a dark outline, for names and counts. */
export function tagTexture(text: string, color: string) {
  const key = `tag:${text}:${color}`;
  const cached = textureCache.get(key);
  if (cached) return cached;
  const canvas = document.createElement('canvas');
  const ctx = canvas.getContext('2d')!;
  ctx.font = '700 40px system-ui, sans-serif';
  canvas.width = Math.ceil(ctx.measureText(text).width) + 40;
  canvas.height = 60;
  ctx.font = '700 40px system-ui, sans-serif';
  ctx.textBaseline = 'middle';
  ctx.lineWidth = 8;
  ctx.lineJoin = 'round';
  ctx.strokeStyle = 'rgba(15,23,42,0.85)';
  ctx.strokeText(text, 20, 32);
  ctx.fillStyle = color;
  ctx.fillText(text, 20, 32);
  const texture = new THREE.CanvasTexture(canvas);
  texture.colorSpace = THREE.SRGBColorSpace;
  textureCache.set(key, texture);
  return texture;
}

export function tagSprite(text: string, color: string, width = 2.2) {
  const sprite = new THREE.Sprite(
    new THREE.SpriteMaterial({ map: tagTexture(text, color), depthTest: false, transparent: true }),
  );
  sprite.scale.set(width, width / 4, 1);
  return sprite;
}

export function retag(sprite: THREE.Sprite, text: string, color: string) {
  const material = sprite.material as THREE.SpriteMaterial;
  const next = tagTexture(text, color);
  if (material.map !== next) {
    material.map = next;
    material.needsUpdate = true;
    sprite.scale.x = (next.image.width / next.image.height) * sprite.scale.y;
  }
}

export function wrap(text: string, max = 14): string[] {
  const words = text.split(' ');
  const lines: string[] = [];
  let line = '';
  for (const word of words) {
    if ((line + ' ' + word).trim().length > max && line) {
      lines.push(line);
      line = word;
    } else {
      line = (line + ' ' + word).trim();
    }
  }
  if (line) lines.push(line);
  return lines;
}

export const backMaterial = new THREE.MeshStandardMaterial({ color: '#3b2a1e', roughness: 0.8 });
const edgeMaterial = new THREE.MeshStandardMaterial({ color: '#e7dcc6', roughness: 0.9 });
const cardGeometry = new THREE.BoxGeometry(CARD_W, 0.04, CARD_H);

/** A playing card: face texture on top, or on the underside when face down. */
export function cardMesh(faceTexture: THREE.Texture, faceDown: boolean) {
  const face = new THREE.MeshStandardMaterial({ map: faceTexture, roughness: 0.7 });
  const materials = [
    edgeMaterial,
    edgeMaterial,
    faceDown ? backMaterial : face,
    faceDown ? face : backMaterial,
    edgeMaterial,
    edgeMaterial,
  ];
  const mesh = new THREE.Mesh(cardGeometry, materials);
  mesh.castShadow = true;
  return mesh;
}

export function disposeCard(mesh: THREE.Mesh) {
  const materials = mesh.material as THREE.Material[];
  materials[2].dispose();
  materials[3].dispose();
}

export function seatAngle(i: number, n: number) {
  return (i / Math.max(1, n)) * Math.PI * 2;
}

export function spot(angle: number, radius: number) {
  return new THREE.Vector3(Math.sin(angle) * radius, 0, Math.cos(angle) * radius);
}

/** The direction along the table edge at a seat, for laying cards in a row. */
export function across(angle: number) {
  return new THREE.Vector3(Math.cos(angle), 0, -Math.sin(angle));
}

/** Floor, rug, the round table, its cloth and a candle in the middle. */
export function buildRoundTable(group: THREE.Group) {
  const floor = new THREE.Mesh(
    new THREE.CylinderGeometry(12.5, 12.5, 0.6, 48),
    new THREE.MeshStandardMaterial({ color: '#6b4f3a', roughness: 0.95 }),
  );
  floor.position.y = -0.3;
  floor.receiveShadow = true;
  const rug = new THREE.Mesh(
    new THREE.CircleGeometry(11.5, 48),
    new THREE.MeshStandardMaterial({ color: '#7f2d3a', roughness: 1 }),
  );
  rug.rotation.x = -Math.PI / 2;
  rug.position.y = 0.01;
  rug.receiveShadow = true;
  const table = new THREE.Mesh(
    new THREE.CylinderGeometry(TABLE_RADIUS, TABLE_RADIUS * 0.92, 0.5, 48),
    new THREE.MeshStandardMaterial({ color: '#8a5a32', roughness: 0.6 }),
  );
  table.position.y = TABLE_TOP - 0.25;
  table.castShadow = true;
  table.receiveShadow = true;
  const pedestal = new THREE.Mesh(
    new THREE.CylinderGeometry(1.2, 1.8, TABLE_TOP - 0.5, 24),
    new THREE.MeshStandardMaterial({ color: '#5b3a20' }),
  );
  pedestal.position.y = (TABLE_TOP - 0.5) / 2;
  const cloth = new THREE.Mesh(
    new THREE.CircleGeometry(TABLE_RADIUS - 0.5, 48),
    new THREE.MeshStandardMaterial({ color: '#1f3d2b', roughness: 1 }),
  );
  cloth.rotation.x = -Math.PI / 2;
  cloth.position.y = TABLE_TOP + 0.005;
  cloth.receiveShadow = true;
  const candle = new THREE.Mesh(
    new THREE.CylinderGeometry(0.12, 0.14, 0.7, 12),
    new THREE.MeshStandardMaterial({ color: '#f5e6c8' }),
  );
  candle.position.set(0, TABLE_TOP + 0.35, 0);
  const flame = new THREE.PointLight('#ffb347', 6, 14, 1.6);
  flame.position.set(0, TABLE_TOP + 0.9, 0);
  const wick = new THREE.Mesh(
    new THREE.SphereGeometry(0.08, 8, 8),
    new THREE.MeshBasicMaterial({ color: '#ffd166' }),
  );
  wick.position.copy(flame.position).y -= 0.15;
  group.add(floor, rug, table, pedestal, cloth, candle, flame, wick);
}

export type SeatEntry = { seat: number; characterId: string; name: string; alive: boolean };

type Seat = {
  rig: CharacterRig | null;
  characterId: string;
  tag: THREE.Sprite;
  hit: THREE.Mesh;
  ring: THREE.Mesh<THREE.RingGeometry, THREE.MeshBasicMaterial>;
  angle: number;
  alive: boolean;
};

/**
 * The people round a table: a character per seat facing the middle, a name
 * tag, an invisible cylinder to click and a ring that lights up on their turn.
 */
export class TableSeats {
  private seats = new Map<number, Seat>();
  private count = 0;
  highlight = new Map<number, string>();

  constructor(private group: THREE.Group) {}

  get size() {
    return this.count;
  }

  angle(seat: number | null) {
    if (seat === null) return 0;
    return this.seats.get(seat)?.angle ?? seatAngle(seat, Math.max(1, this.count));
  }

  hits(): THREE.Object3D[] {
    return [...this.seats.values()].map((s) => s.hit);
  }

  sync(entries: SeatEntry[], roster: Map<string, CharacterDef>) {
    const n = entries.length;
    if (n !== this.count) {
      this.count = n;
      for (const seat of this.seats.values()) this.drop(seat);
      this.seats.clear();
    }
    for (const entry of entries) {
      const angle = seatAngle(entry.seat, n);
      let seat = this.seats.get(entry.seat);
      if (seat && seat.characterId !== entry.characterId) {
        this.drop(seat);
        seat = undefined;
      }
      if (!seat) {
        const def = roster.get(entry.characterId) ?? roster.values().next().value;
        const rig = def ? buildCharacter(def) : null;
        if (rig) {
          rig.group.scale.setScalar(0.9);
          this.group.add(rig.group);
        }
        const tag = tagSprite(entry.name, '#ffffff');
        const hit = new THREE.Mesh(
          new THREE.CylinderGeometry(1.1, 1.1, 3, 8),
          new THREE.MeshBasicMaterial({ visible: false }),
        );
        hit.userData = { seat: entry.seat };
        const ring = new THREE.Mesh(
          new THREE.RingGeometry(1.0, 1.35, 32),
          new THREE.MeshBasicMaterial({
            color: '#fbbf24',
            transparent: true,
            opacity: 0.85,
            side: THREE.DoubleSide,
          }),
        );
        ring.rotation.x = -Math.PI / 2;
        ring.visible = false;
        this.group.add(tag, hit, ring);
        seat = { rig, characterId: entry.characterId, tag, hit, ring, angle, alive: entry.alive };
        this.seats.set(entry.seat, seat);
      }
      seat.angle = angle;
      seat.alive = entry.alive;
      const at = spot(angle, SEAT_RADIUS);
      if (seat.rig) {
        seat.rig.group.position.copy(at);
        seat.rig.group.rotation.y = Math.atan2(-at.x, -at.z);
      }
      seat.tag.position.set(at.x, 2.6, at.z);
      retag(seat.tag, entry.name, entry.alive ? '#ffffff' : '#94a3b8');
      seat.hit.position.set(at.x, 1.5, at.z);
      seat.ring.position.set(at.x, 0.03, at.z);
    }
    for (const [key, seat] of [...this.seats]) {
      if (!entries.some((s) => s.seat === key)) {
        this.drop(seat);
        this.seats.delete(key);
      }
    }
  }

  update(time: number, turnSeat: number | null) {
    for (const [seat, entry] of this.seats) {
      const pose: Pose = !entry.alive ? 'fall' : seat === turnSeat ? 'cheer' : 'idle';
      entry.rig?.animate(pose, time, 0);
      const color = this.highlight.get(seat);
      entry.ring.visible = !!color;
      if (color) {
        entry.ring.material.color.set(color);
        entry.ring.material.opacity = 0.6 + Math.sin(time * 4) * 0.25;
      }
    }
  }

  private drop(seat: Seat) {
    seat.rig?.dispose();
    if (seat.rig) this.group.remove(seat.rig.group);
    this.group.remove(seat.tag, seat.hit, seat.ring);
    seat.tag.material.dispose();
    seat.hit.geometry.dispose();
    (seat.hit.material as THREE.Material).dispose();
    seat.ring.geometry.dispose();
    seat.ring.material.dispose();
  }

  dispose() {
    for (const seat of this.seats.values()) this.drop(seat);
    this.seats.clear();
    this.count = 0;
  }
}
