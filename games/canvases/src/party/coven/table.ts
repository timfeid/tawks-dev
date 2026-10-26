import * as THREE from 'three';
import type { CharacterDef, CovenCard, CovenView, Member } from '../protocol';
import { CARDS, COLOR_HEX, VERDICTS } from './rules';
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

const VERDICT_RADIUS = 4.9;
const PILE_RADIUS = 3.3;
const FLIP_SECONDS = 0.45;

type VerdictCard = {
  group: THREE.Group;
  mesh: THREE.Mesh;
  revealed: boolean;
  flip: number;
  label: THREE.Sprite | null;
  face: string;
};

type Pile = { group: THREE.Group; key: string };

function playingCardTexture(card: CovenCard) {
  const spec = CARDS[card];
  return textTexture([spec.title, ...wrap(spec.text, 16)], '#f8fafc', COLOR_HEX[spec.color]);
}

/**
 * The Coven table: everyone seated round it, their verdict cards laid face
 * down in front of them, the accusations and blue cards piled nearer the
 * middle, the deck and the last discard at the centre.
 */
export class CovenTable {
  readonly group = new THREE.Group();
  private seats: TableSeats;
  private verdicts = new Map<string, VerdictCard>();
  private piles = new Map<number, Pile>();
  private deck: THREE.Mesh;
  private discard: THREE.Mesh | null = null;
  private discardKey = '';
  private cat: THREE.Group;
  private turnSeat: number | null = null;

  constructor(scene: THREE.Scene) {
    this.group.visible = false;
    scene.add(this.group);
    buildRoundTable(this.group);
    this.seats = new TableSeats(this.group);
    this.deck = new THREE.Mesh(
      new THREE.BoxGeometry(CARD_W, 0.04, 1.3),
      new THREE.MeshStandardMaterial({ color: '#3b2a1e', roughness: 0.8 }),
    );
    this.deck.position.set(-0.9, TABLE_TOP + 0.02, 0.6);
    this.deck.castShadow = true;
    this.group.add(this.deck);

    this.cat = new THREE.Group();
    const fur = new THREE.MeshStandardMaterial({ color: '#0b0b12', roughness: 0.9 });
    const body = new THREE.Mesh(new THREE.SphereGeometry(0.34, 14, 12), fur);
    body.scale.set(1.2, 0.8, 1);
    body.position.y = 0.26;
    const head = new THREE.Mesh(new THREE.SphereGeometry(0.22, 12, 10), fur);
    head.position.set(0.36, 0.5, 0);
    const ears = [-1, 1].map((side) => {
      const ear = new THREE.Mesh(new THREE.ConeGeometry(0.08, 0.18, 6), fur);
      ear.position.set(0.4, 0.7, side * 0.12);
      return ear;
    });
    const eyes = [-1, 1].map((side) => {
      const eye = new THREE.Mesh(
        new THREE.SphereGeometry(0.04, 6, 6),
        new THREE.MeshBasicMaterial({ color: '#facc15' }),
      );
      eye.position.set(0.55, 0.54, side * 0.08);
      return eye;
    });
    this.cat.add(body, head, ...ears, ...eyes);
    this.cat.visible = false;
    this.group.add(this.cat);
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

  /** The seat or verdict card under the pointer, if any. */
  pick(ray: THREE.Raycaster): { seat: number; index?: number } | null {
    const targets: THREE.Object3D[] = [
      ...this.seats.hits(),
      ...[...this.verdicts.values()].map((c) => c.mesh),
    ];
    const hit = ray.intersectObjects(targets, false)[0];
    if (!hit) return null;
    const data = hit.object.userData as { seat?: number; index?: number };
    if (data.seat === undefined) return null;
    return { seat: data.seat, index: data.index };
  }

  /** Lays the table for a view, or seats the lobby's members round an empty one. */
  sync(view: CovenView | null, members: Member[], roster: Map<string, CharacterDef>) {
    const seated = view
      ? view.seats.map((s, i) => ({
          seat: i,
          characterId: s.character ?? '',
          name: s.name,
          alive: view.players[i]?.alive ?? true,
        }))
      : [...members]
          .sort((a, b) => a.seat - b.seat)
          .map((m, i) => ({ seat: i, characterId: m.character, name: m.name, alive: true }));
    this.seats.sync(seated, roster);
    this.syncCards(view);
  }

  private syncCards(view: CovenView | null) {
    const wanted = new Set<string>();
    if (view) {
      const n = view.players.length;
      for (const player of view.players) {
        const angle = seatAngle(player.seat, n);
        const centre = spot(angle, VERDICT_RADIUS);
        const side = across(angle);
        const width = player.verdicts.length;
        player.verdicts.forEach((slot, index) => {
          const key = `${player.seat}:${index}`;
          wanted.add(key);
          const face = slot.card ? VERDICTS[slot.card].title : '';
          let card = this.verdicts.get(key);
          if (card && card.face !== face) {
            this.dropVerdict(card);
            this.verdicts.delete(key);
            card = undefined;
          }
          if (!card) {
            const texture = slot.card
              ? textTexture(
                  ['Verdict', VERDICTS[slot.card].title.toUpperCase()],
                  VERDICTS[slot.card].color,
                  '#1e1b2e',
                )
              : textTexture(['Verdict'], '#e2e8f0', '#1e1b2e');
            const mesh = cardMesh(texture, true);
            mesh.userData = { seat: player.seat, index };
            const group = new THREE.Group();
            group.add(mesh);
            this.group.add(group);
            let label: THREE.Sprite | null = null;
            if (slot.card && !slot.revealed) {
              label = tagSprite(VERDICTS[slot.card].title, VERDICTS[slot.card].color, 1.5);
              label.position.y = 0.5;
              group.add(label);
            }
            card = {
              group,
              mesh,
              revealed: slot.revealed,
              flip: slot.revealed ? 1 : 0,
              label,
              face,
            };
            this.verdicts.set(key, card);
          }
          if (slot.revealed && !card.revealed) {
            card.revealed = true;
            if (card.label) card.label.visible = false;
          }
          const offset = (index - (width - 1) / 2) * (CARD_W + 0.16);
          card.group.position
            .copy(centre)
            .addScaledVector(side, offset)
            .setY(TABLE_TOP + 0.03);
          card.group.rotation.y = angle;
        });
      }
    }
    for (const [key, card] of [...this.verdicts]) {
      if (!wanted.has(key)) {
        this.dropVerdict(card);
        this.verdicts.delete(key);
      }
    }
    this.syncPiles(view);
    const count = view?.deck ?? 0;
    this.deck.scale.y = Math.max(1, count * 1.2);
    this.deck.position.y = TABLE_TOP + 0.02 * this.deck.scale.y;
    this.deck.visible = count > 0;
    const top = view?.discard_top ?? null;
    const discardKey = top ?? '';
    if (discardKey !== this.discardKey) {
      this.discardKey = discardKey;
      if (this.discard) {
        this.group.remove(this.discard);
        disposeCard(this.discard);
        this.discard = null;
      }
      if (top) {
        this.discard = cardMesh(playingCardTexture(top), false);
        this.discard.position.set(0.9, TABLE_TOP + 0.03, 0.6);
        this.discard.rotation.y = 0.08;
        this.group.add(this.discard);
      }
    }
    const catSeat = view?.familiar ?? null;
    this.cat.visible = catSeat !== null;
    if (catSeat !== null) {
      const angle = this.seats.angle(catSeat);
      const at = spot(angle, PILE_RADIUS).addScaledVector(across(angle), 1.5);
      this.cat.position.set(at.x, TABLE_TOP, at.z);
      this.cat.rotation.y = angle + Math.PI / 2;
    }
    const phase = view?.phase;
    this.turnSeat =
      phase?.kind === 'turn' ? phase.seat : phase?.kind === 'reveal' ? phase.chooser : null;
    const accused = phase?.kind === 'reveal' ? phase.accused : null;
    this.seats.highlight.clear();
    if (this.turnSeat !== null) this.seats.highlight.set(this.turnSeat, '#fbbf24');
    if (accused !== null) this.seats.highlight.set(accused, '#ef4444');
  }

  private syncPiles(view: CovenView | null) {
    const wanted = new Set<number>();
    if (view) {
      const n = view.players.length;
      for (const player of view.players) {
        const reds = player.table.filter((c) => CARDS[c].color === 'red');
        const blues = player.table.filter((c) => CARDS[c].color === 'blue' && c !== 'familiar');
        if (!reds.length && !blues.length) continue;
        wanted.add(player.seat);
        const key = `${player.accusations}:${blues.join(',')}`;
        let pile = this.piles.get(player.seat);
        if (pile && pile.key !== key) {
          this.dropPile(pile);
          pile = undefined;
        }
        if (!pile) {
          const group = new THREE.Group();
          const angle = seatAngle(player.seat, n);
          const centre = spot(angle, PILE_RADIUS);
          const side = across(angle);
          let slot = -blues.length / 2;
          if (reds.length) {
            const stack = cardMesh(
              textTexture(['Accused', `${player.accusations}`], '#fff1f2', COLOR_HEX.red),
              false,
            );
            stack.position
              .copy(centre)
              .addScaledVector(side, slot * (CARD_W + 0.1))
              .setY(TABLE_TOP + 0.03 + reds.length * 0.01);
            stack.scale.y = Math.max(1, reds.length);
            stack.rotation.y = angle;
            group.add(stack);
            slot += 1;
          }
          for (const blue of blues) {
            const mesh = cardMesh(playingCardTexture(blue), false);
            mesh.position
              .copy(centre)
              .addScaledVector(side, slot * (CARD_W + 0.1))
              .setY(TABLE_TOP + 0.03);
            mesh.rotation.y = angle + 0.05;
            group.add(mesh);
            slot += 1;
          }
          this.group.add(group);
          pile = { group, key };
          this.piles.set(player.seat, pile);
        }
      }
    }
    for (const [seat, pile] of [...this.piles]) {
      if (!wanted.has(seat)) {
        this.dropPile(pile);
        this.piles.delete(seat);
      }
    }
  }

  private dropPile(pile: Pile) {
    this.group.remove(pile.group);
    pile.group.traverse((obj) => {
      if (obj instanceof THREE.Mesh && Array.isArray(obj.material)) disposeCard(obj);
    });
  }

  private dropVerdict(card: VerdictCard) {
    this.group.remove(card.group);
    disposeCard(card.mesh);
    card.label?.material.dispose();
  }

  update(time: number, dt: number) {
    for (const card of this.verdicts.values()) {
      if (card.revealed && card.flip < 1) {
        card.flip = Math.min(1, card.flip + dt / FLIP_SECONDS);
        const t = card.flip;
        card.mesh.rotation.x = Math.PI * t;
        card.mesh.position.y = Math.sin(Math.PI * t) * 0.6;
      }
    }
    this.seats.update(time, this.turnSeat);
    if (this.cat.visible) this.cat.position.y = TABLE_TOP + Math.sin(time * 2) * 0.02;
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
