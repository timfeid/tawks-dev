import * as THREE from 'three';
import { BaseZone } from './Zone';
import type { CardActor } from '../entities/CardActor';

/**
 * Drafted cards laid out like the deck-builder wings on both sides.
 */
const WING_RADIUS = 4.15;
const WING_ARC = Math.PI * 0.54;
const LEFT_WING_START_ANGLE = Math.PI * 0.38;
const WING_START_ANGLE = Math.PI * 1.12;
const WING_Y = 0.018;
const MAX_VISIBLE = 90;
const MAX_ROWS_PER_WING = 7;
const ROW_RADIUS_STEP = 0.58;
const ROW_LIFT_STEP = 0.004;
const SIDE_ANGLE_SHIFT = Math.PI * 0.05;
const ZIG_ZAG_ANGLE_OFFSET = Math.PI * 0.028;

export class DraftSideZone extends BaseZone {
  private draftedIds: string[] = [];

  constructor() {
    super('draft_side');
  }

  public setDrafted(cardIds: string[]): void {
    this.draftedIds = cardIds;
  }

  public update(cards: any[]): void {
    this.actors.clear();
  }

  public layout(allActors: Map<string, CardActor>): void {
    this.actors.clear();
    const visibleIds = this.draftedIds.slice(-MAX_VISIBLE);
    const count = visibleIds.length;
    if (count === 0) return;

    const splitIndex = Math.ceil(count / 2);
    const leftIds = visibleIds.slice(0, splitIndex);
    const rightIds = visibleIds.slice(splitIndex);
    const cardsPerRow =
      count > 78 ? 8 : count > 60 ? 7 : count > 36 ? 6 : 5;
    const cardScale = count > 78 ? 0.34 : count > 60 ? 0.39 : count > 36 ? 0.45 : 0.5;

    const layoutWing = (
      cardIds: string[],
      startAngle: number,
      sideDirection: 1 | -1,
    ) => {
      const wingCount = cardIds.length;
      if (wingCount === 0) return;
      const totalRows = Math.min(
        MAX_ROWS_PER_WING,
        Math.max(1, Math.ceil(wingCount / cardsPerRow)),
      );

      for (let rowIndex = 0; rowIndex < totalRows; rowIndex++) {
        const rowIds = cardIds.slice(
          rowIndex * cardsPerRow,
          (rowIndex + 1) * cardsPerRow,
        );
        const cardsInRow = rowIds.length;
        if (cardsInRow === 0) continue;

        const rowArc = Math.min(
          WING_ARC,
          Math.PI * (0.18 + 0.06 * Math.max(0, cardsInRow - 1)),
        );
        const angleStep =
          cardsInRow > 1 ? rowArc / Math.max(1, cardsInRow - 1) : rowArc;
        const rowRadius = WING_RADIUS + rowIndex * ROW_RADIUS_STEP;
        const rowLift = rowIndex * ROW_LIFT_STEP;
        const rowStartAngle =
          startAngle +
          sideDirection * SIDE_ANGLE_SHIFT +
          (WING_ARC - rowArc) * 0.5 +
          (rowIndex % 2 === 1 ? sideDirection * ZIG_ZAG_ANGLE_OFFSET : 0);

        for (let i = 0; i < cardsInRow; i++) {
          const cardId = rowIds[i];
          const actor = allActors.get(cardId);
          if (!actor) continue;

          const globalIndex = rowIndex * cardsPerRow + i;
          this.actors.set(cardId, actor);
          actor.setZone('draft_side');
          actor.setFlatLayout(true);

          const angle = rowStartAngle + i * angleStep;
          actor.setTarget(
            new THREE.Vector3(
              Math.cos(angle) * rowRadius,
              WING_Y + rowLift + globalIndex * 0.001,
              Math.sin(angle) * rowRadius,
            ),
            new THREE.Euler(
              -Math.PI / 2 + (globalIndex % 2 === 0 ? 0.01 : -0.01),
              0,
              -angle +
                Math.PI / 2 +
                (globalIndex % 2 === 0 ? 0.005 : -0.005),
            ),
            cardScale,
            0.12,
          );
        }
      }
    };

    layoutWing(leftIds, LEFT_WING_START_ANGLE, 1);
    layoutWing(rightIds, WING_START_ANGLE, -1);
  }

  public getDropIndex(
    _point: THREE.Vector3,
    _viewerId: string | null,
  ): number {
    return -1;
  }

  public getCardIds(_viewerId: string | null): string[] {
    return [...this.draftedIds];
  }

  public getSceneObjects(): THREE.Object3D[] {
    return [];
  }
}
