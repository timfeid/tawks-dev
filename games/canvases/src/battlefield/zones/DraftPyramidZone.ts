import * as THREE from 'three';
import { BaseZone } from './Zone';
import type { CardActor } from '../entities/CardActor';

/**
 * Hex-pack layout for draft mode.
 *
 * Index mapping (19 slots):
 *       [0]  [1]  [2]
 *    [3]  [4]  [5]  [6]
 * [7]  [8]  [9] [10] [11]
 *   [12] [13] [14] [15]
 *      [16] [17] [18]
 */

const CARD_H_SPACING = 1.0;
const CARD_V_SPACING = 1.15;
const BASE_Y = 0.03;
const AVAILABLE_LIFT = 0.04;

// Row definitions: [startIndex, count]
const ROWS: [number, number][] = [
  [0, 3],
  [3, 4],
  [7, 5],
  [12, 4],
  [16, 3],
];

function pyramidPosition(index: number): { x: number; z: number; row: number } {
  for (let r = 0; r < ROWS.length; r++) {
    const [start, count] = ROWS[r];
    if (index >= start && index < start + count) {
      const col = index - start;
      const halfWidth = ((count - 1) * CARD_H_SPACING) / 2;
      const z = col * CARD_H_SPACING - halfWidth;
      const x = (r - 2) * CARD_V_SPACING;
      return { x, z, row: r };
    }
  }
  return { x: 0, z: 0, row: 0 };
}

export class DraftPyramidZone extends BaseZone {
  private pyramidCardIds: (string | null)[] = new Array(19).fill(null);
  private availableIndices = new Set<number>();
  private cardIdToIndex = new Map<string, number>();
  private group = new THREE.Group();
  private time = 0;
  public inspectedCardId: string | null = null;

  constructor() {
    super('draft_pyramid');
  }

  public setPyramid(
    cardIds: (string | null)[],
    availableIndices: number[],
  ): void {
    this.pyramidCardIds = cardIds;
    this.availableIndices = new Set(availableIndices);
    this.cardIdToIndex.clear();
    for (let i = 0; i < cardIds.length; i++) {
      if (cardIds[i]) this.cardIdToIndex.set(cardIds[i]!, i);
    }
  }

  public pyramidIndexForCardId(cardId: string): number | undefined {
    return this.cardIdToIndex.get(cardId);
  }

  public isAvailable(index: number): boolean {
    return this.availableIndices.has(index);
  }

  public update(cards: any[]): void {
    // Cards are synced via ZoneSyncController — this stores the card IDs for the zone
    const ids: string[] = [];
    for (const id of this.pyramidCardIds) {
      if (id) ids.push(id);
    }
    // Update actor zone assignment
    this.actors.clear();
  }

  public layout(allActors: Map<string, CardActor>): void {
    this.actors.clear();
    for (let i = 0; i < this.pyramidCardIds.length; i++) {
      const cardId = this.pyramidCardIds[i];
      if (!cardId) continue;

      const actor = allActors.get(cardId);
      if (!actor) continue;

      this.actors.set(cardId, actor);
      actor.setZone('draft_pyramid');

      const pos = pyramidPosition(i);
      const isAvail = this.availableIndices.has(i);
      const y = isAvail ? BASE_Y + AVAILABLE_LIFT : BASE_Y;

      // Subtle bob for available cards
      const bob = isAvail ? Math.sin(this.time * 2 + i) * 0.015 : 0;

      actor.setTarget(
        new THREE.Vector3(pos.x, y + bob, pos.z),
        new THREE.Euler(-Math.PI / 2, 0, Math.PI / 2),
        isAvail ? 0.9 : 0.72,
        0.12,
      );

      // Grayscale unavailable cards (skip if inspected)
      const isInspected = cardId === this.inspectedCardId;
      const grayTarget = (isAvail || isInspected) ? 0.0 : 1.0;
      actor.group.traverse((child) => {
        if (!(child instanceof THREE.Mesh) || !child.material) return;
        const mat = child.material as THREE.ShaderMaterial;
        if (mat.uniforms?.grayscale != null) {
          mat.uniforms.grayscale.value = THREE.MathUtils.lerp(
            mat.uniforms.grayscale.value, grayTarget, 0.08,
          );
        }
      });
    }
  }

  public tick(delta: number, time: number): void {
    this.time = time;
  }

  public getDropIndex(
    _point: THREE.Vector3,
    _viewerId: string | null,
  ): number {
    return -1;
  }

  public getCardIds(_viewerId: string | null): string[] {
    return this.pyramidCardIds.filter((id): id is string => id !== null);
  }

  public getSceneObjects(): THREE.Object3D[] {
    return [this.group];
  }
}
