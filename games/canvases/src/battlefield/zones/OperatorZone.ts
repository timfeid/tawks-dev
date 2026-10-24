import * as THREE from 'three';
import { BaseZone } from './Zone';
import type { CardActor } from '../entities/CardActor';
import type { CardView, PlayerState } from '../types';

export class OperatorZone extends BaseZone {
  private players: PlayerState[] = [];
  private cardW = 0.75;
  private cardH = 1.1;
  private baseTableRadius = 10.0;
  private tableRadius = 10.0;

  constructor() {
    super('operator');
  }

  public update(_cards: CardView[]) {
    // Player-driven zone; state is synced through updateState().
  }

  public updateState(players: PlayerState[]) {
    this.players = players;
  }

  /**
   * Canonical entry point — set the effective outer-ring radius directly.
   * Caller should pass `tableManager.getEffectiveRadius()` so every zone
   * agrees on the table size instead of each re-multiplying base × scale.
   */
  public setTableRadius(radius: number): void {
    this.tableRadius = radius;
  }

  /** Backwards-compat shim (caller used to pass a scale factor). */
  public setTableScale(scale: number): void {
    this.tableRadius = this.baseTableRadius * scale;
  }

  public layout(allActors: Map<string, CardActor>) {
    const count = this.players.length;
    if (count === 0) return;

    this.players.forEach((player, index) => {
      const operatorCards = player.operator_zone || [];
      if (operatorCards.length === 0) return;

      const seatAngle = (index / count) * Math.PI * 2;
      const forward = new THREE.Vector3(Math.cos(seatAngle), 0, Math.sin(seatAngle));
      const tangent = new THREE.Vector3(-Math.sin(seatAngle), 0, Math.cos(seatAngle));
      
      const sectionOuter = this.tableRadius + 0.15 - this.cardH * 0.5 + 0.2;
      const topEdgeRadius = sectionOuter - 1.5;
      const centerRadius = topEdgeRadius + this.cardH / 2;
      const pileOut = 0.3;

      // Position operator zone on the left side of the player area (opposite to deck/discard)
      const basePos = forward
        .clone()
        .add(tangent.clone().multiplyScalar(-0.45))
        .normalize()
        .multiplyScalar(centerRadius)
        .add(forward.clone().multiplyScalar(pileOut));

      operatorCards.forEach((card, i) => {
        const actor = allActors.get(card.id);
        if (!actor) return;

        actor.setZone('operator');
        actor.setFlatLayout(true);

        const pos = basePos.clone();
        pos.y = 0.02 + i * 0.005;
        
        // Offset multiple operators slightly
        const offset = tangent.clone().multiplyScalar(i * 0.15);
        pos.add(offset);

        const rot = new THREE.Euler(-Math.PI / 2, 0, -seatAngle + Math.PI / 2);
        actor.setTarget(pos, rot, 0.6, 0.2);
      });
    });
  }

  public getCardIds(viewerId: string | null): string[] {
    return this.players.flatMap(p => (p.operator_zone || []).map(c => c.id));
  }

  public getDropIndex(_point: THREE.Vector3, _viewerId: string | null): number {
    return 0;
  }
}
