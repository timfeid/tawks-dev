import * as THREE from 'three';
import type { CardView } from '../types';
import { CardActor } from '../entities/CardActor';
import type { CardFactory } from '../entities/CardFactory';

/**
 * Camera-attached hover inspect panel — a scaled-up copy of the hovered
 * card, rendered with the same CardActor/texture used on the battlefield.
 * All the inspect info (attached_to, modifiers, owner, etc.) is baked into
 * the card texture itself in textures.ts, so this panel doesn't need an
 * extra info sprite — it's just the card, blown up.
 */
export class InspectPanel {
  public group = new THREE.Group();
  private cardActor: CardActor | null = null;
  private currentCardId: string | null = null;
  private currentCardRev = '';

  constructor(
    private camera: THREE.Camera,
    private factory: CardFactory,
  ) {
    this.group.visible = false;
    this.camera.add(this.group);
    this.group.position.set(4.2, 0, -6.5);
  }

  public updateAnchor(camera: THREE.PerspectiveCamera): void {
    const z = 6.5;
    const vFov = (camera.fov * Math.PI) / 180;
    const halfH = Math.tan(vFov / 2) * z;
    const halfW = halfH * camera.aspect;
    const rightEdge = halfW - 0.25;
    const contentHalfW = 0.95; // approx half-width of the scaled card
    this.group.position.x = rightEdge - contentHalfW;
    // Lift toward the top of the viewport so the preview doesn't hide the
    // player's own hand / board.
    this.group.position.y = halfH * 0.25;
    this.group.position.z = -z;
  }

  public setCard(card: CardView | null): void {
    if (!card) {
      this.group.visible = false;
      this.currentCardId = null;
      this.currentCardRev = '';
      return;
    }
    const rev = this.cardRev(card);
    if (card.id === this.currentCardId && rev === this.currentCardRev) {
      this.group.visible = true;
      return;
    }
    this.currentCardId = card.id;
    this.currentCardRev = rev;
    this.rebuild(card);
    this.group.visible = true;
  }

  public tick(delta: number, time: number): void {
    if (!this.group.visible) return;
    this.cardActor?.tick(delta, time);
  }

  public dispose(): void {
    this.camera.remove(this.group);
    if (this.cardActor) {
      this.group.remove(this.cardActor.group);
    }
  }

  private cardRev(card: CardView): string {
    const c = card as any;
    return [
      card.name,
      card.description,
      card.auto_description,
      card.power ?? '',
      card.toughness ?? '',
      c.base_power ?? '',
      c.base_toughness ?? '',
      (card.kind_label ?? card.kind_tag) || '',
      c.attached_to_name ?? '',
      c.remembered_player_name ?? '',
      c.owner_name ?? '',
      c.loyalty ?? '',
      c.counters ?? '',
      c.damage ?? '',
      ((c.stat_modifiers as any[]) ?? [])
        .map(
          (m) =>
            `${m.label}:${m.power_mod}/${m.toughness_mod}:${(m.granted_keywords ?? []).join('+')}`,
        )
        .join(','),
    ].join('|');
  }

  private rebuild(card: CardView): void {
    if (this.cardActor) {
      this.group.remove(this.cardActor.group);
      this.cardActor = null;
    }
    const actor = new CardActor(card, this.factory);
    actor.setZone('hand');
    actor.setFlatLayout(false);
    actor.disableSnap();
    const cardScale = 2.5; // blown-up inspect copy (slightly smaller than before)
    const pos = new THREE.Vector3(0, 0, 0);
    const rot = new THREE.Euler(0, 0, 0);
    actor.setTarget(pos, rot, cardScale, 1.0);
    actor.group.position.copy(pos);
    actor.group.rotation.copy(rot);
    actor.group.scale.setScalar(cardScale);
    this.group.add(actor.group);
    this.cardActor = actor;
  }
}
