import * as THREE from 'three';
import type { CharacterDef, Member } from '../protocol';
import {
  CARD_W,
  TABLE_TOP,
  TableSeats,
  across,
  buildRoundTable,
  cardMesh,
  disposeCard,
  seatAngle,
  spot,
  tagSprite,
  textTexture,
  wrap,
} from '../table/props';
import { GROUP_INFO, type LandlordView, card, cardColor, tableText, isProperty } from './rules';

const SETS_RADIUS = 4.6;
const BANK_RADIUS = 3.0;

type Spread = { group: THREE.Group; key: string };

function faceTexture(id: number) {
  const spec = card(id);
  const dark = ['#facc15', '#7dd3fc', '#a3e635', '#fb923c'].includes(cardColor(spec));
  return textTexture(
    [spec.title, `$${spec.value}`, ...wrap(tableText(spec), 16)],
    dark ? '#0f172a' : '#f8fafc',
    cardColor(spec),
  );
}

/**
 * The Landlord table: each seat's districts fanned out in front of them as
 * coloured stacks, the bank as a pile of face-down cards with a total, and
 * the deck and discard in the middle.
 */
export class LandlordTable {
  readonly group = new THREE.Group();
  private seats: TableSeats;
  private spreads = new Map<number, Spread>();
  private deck: THREE.Mesh;
  private discard: THREE.Mesh | null = null;
  private discardKey = -1;
  private turnSeat: number | null = null;

  constructor(scene: THREE.Scene) {
    this.group.visible = false;
    scene.add(this.group);
    buildRoundTable(this.group);
    this.seats = new TableSeats(this.group);
    this.deck = new THREE.Mesh(
      new THREE.BoxGeometry(CARD_W, 0.04, 1.3),
      new THREE.MeshStandardMaterial({ color: '#1e3a5f', roughness: 0.8 }),
    );
    this.deck.position.set(-0.9, TABLE_TOP + 0.02, 0.6);
    this.deck.castShadow = true;
    this.group.add(this.deck);
  }

  set visible(value: boolean) {
    this.group.visible = value;
  }

  get visible() {
    return this.group.visible;
  }

  seatAngle(seat: number | null) {
    return this.seats.angle(seat);
  }

  pick(ray: THREE.Raycaster): { seat: number } | null {
    const hit = ray.intersectObjects(this.seats.hits(), false)[0];
    const seat = (hit?.object.userData as { seat?: number } | undefined)?.seat;
    return seat === undefined ? null : { seat };
  }

  sync(view: LandlordView | null, members: Member[], roster: Map<string, CharacterDef>) {
    const seated = view
      ? view.seats.map((s, i) => ({
          seat: i,
          characterId: s.character ?? '',
          name: s.name,
          alive: true,
        }))
      : [...members]
          .sort((a, b) => a.seat - b.seat)
          .map((m, i) => ({ seat: i, characterId: m.character, name: m.name, alive: true }));
    this.seats.sync(seated, roster);
    const wanted = new Set<number>();
    if (view) {
      const n = view.players.length;
      for (const player of view.players) {
        wanted.add(player.seat);
        const key = JSON.stringify([
          player.bank_total,
          player.bank.length,
          player.sets.map((s) => [s.group, s.cards, s.house, s.hotel]),
        ]);
        let spread = this.spreads.get(player.seat);
        if (spread && spread.key !== key) {
          this.dropSpread(spread);
          spread = undefined;
        }
        if (!spread) {
          spread = { group: this.buildSpread(player.seat, n, player), key };
          this.group.add(spread.group);
          this.spreads.set(player.seat, spread);
        }
      }
    }
    for (const [seat, spread] of [...this.spreads]) {
      if (!wanted.has(seat)) {
        this.dropSpread(spread);
        this.spreads.delete(seat);
      }
    }
    const count = view?.deck ?? 0;
    this.deck.scale.y = Math.max(1, count * 1.2);
    this.deck.position.y = TABLE_TOP + 0.02 * this.deck.scale.y;
    this.deck.visible = count > 0;
    const top = view?.discard_top ?? -1;
    if (top !== this.discardKey) {
      this.discardKey = top;
      if (this.discard) {
        this.group.remove(this.discard);
        disposeCard(this.discard);
        this.discard = null;
      }
      if (top >= 0) {
        this.discard = cardMesh(faceTexture(top), false);
        this.discard.position.set(0.9, TABLE_TOP + 0.03, 0.6);
        this.discard.rotation.y = 0.08;
        this.group.add(this.discard);
      }
    }
    const phase = view?.phase;
    this.turnSeat = phase?.kind === 'turn' ? phase.seat : null;
    this.seats.highlight.clear();
    if (this.turnSeat !== null) this.seats.highlight.set(this.turnSeat, '#fbbf24');
    if (phase?.kind === 'respond') this.seats.highlight.set(phase.target, '#a78bfa');
    if (phase?.kind === 'pay') this.seats.highlight.set(phase.from, '#ef4444');
    if (phase?.kind === 'over') this.seats.highlight.set(phase.winner, '#22c55e');
  }

  private buildSpread(seat: number, n: number, player: LandlordView['players'][number]) {
    const group = new THREE.Group();
    const angle = seatAngle(seat, n);
    const side = across(angle);
    const sets = player.sets;
    const setsCentre = spot(angle, SETS_RADIUS);
    sets.forEach((set, i) => {
      const offset = (i - (sets.length - 1) / 2) * (CARD_W + 0.25);
      const base = setsCentre.clone().addScaledVector(side, offset);
      set.cards.forEach((id, j) => {
        const mesh = cardMesh(faceTexture(id), false);
        mesh.position.copy(base).setY(TABLE_TOP + 0.03 + j * 0.05);
        mesh.position.addScaledVector(spot(angle, 1).normalize(), -j * 0.22);
        mesh.rotation.y = angle;
        group.add(mesh);
      });
      const info = GROUP_INFO[set.group];
      const label = tagSprite(
        `${set.cards.length}/${info.size}${set.house ? ' +H' : ''}${set.hotel ? '+T' : ''}`,
        set.complete ? '#4ade80' : '#ffffff',
        1.4,
      );
      label.position.copy(base).setY(TABLE_TOP + 0.9);
      group.add(label);
      if (set.house || set.hotel) {
        const building = new THREE.Mesh(
          new THREE.BoxGeometry(0.35, set.hotel ? 0.7 : 0.35, 0.35),
          new THREE.MeshStandardMaterial({ color: set.hotel ? '#dc2626' : '#16a34a' }),
        );
        building.position.copy(base).setY(TABLE_TOP + 0.2 + (set.hotel ? 0.35 : 0.17));
        building.castShadow = true;
        group.add(building);
      }
    });
    if (player.bank.length) {
      const bankAt = spot(angle, BANK_RADIUS).addScaledVector(side, 1.4);
      const pile = new THREE.Mesh(
        new THREE.BoxGeometry(CARD_W, 0.04, 1.3),
        new THREE.MeshStandardMaterial({ color: '#14532d', roughness: 0.8 }),
      );
      pile.scale.y = Math.max(1, player.bank.length);
      pile.position.copy(bankAt).setY(TABLE_TOP + 0.02 * pile.scale.y);
      pile.rotation.y = angle + 0.2;
      pile.castShadow = true;
      group.add(pile);
      const label = tagSprite(`$${player.bank_total}`, '#86efac', 1.4);
      label.position.copy(bankAt).setY(TABLE_TOP + 0.8);
      group.add(label);
    }
    return group;
  }

  private dropSpread(spread: Spread) {
    this.group.remove(spread.group);
    spread.group.traverse((obj) => {
      if (obj instanceof THREE.Mesh && Array.isArray(obj.material)) disposeCard(obj);
      else if (obj instanceof THREE.Sprite) obj.material.dispose();
    });
  }

  update(time: number) {
    this.seats.update(time, this.turnSeat);
  }

  reset() {
    this.sync(null, [], new Map());
  }

  dispose() {
    this.reset();
    this.seats.dispose();
    this.group.parent?.remove(this.group);
  }
}
