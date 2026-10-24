import * as THREE from 'three';
import { BaseZone } from './Zone';
import type { CardActor } from '../entities/CardActor';
import type { CardView } from '../types';

/**
 * SpotlightZone — renders the top of the stack as a large 3D card floating
 * in the center of the camera view so the player can clearly see what was just
 * cast or what's about to resolve. Hides itself when the stack is empty.
 */
export class SpotlightZone extends BaseZone {
  private spotlightIds: string[] = [];

  // Layout parameters
  private floatHeight = 4.5;
  private floatScale = 0.9;
  private cardSpacing = 0.85;
  // Where the card anchors in the camera's view: distance in front, and how
  // far up from the camera's look point. Kept close to eye level so the
  // pending-play card reads without hovering uncomfortably high.
  private distanceInFront = 7.0;
  private heightAboveCenter = 0.4;

  constructor(private camera?: THREE.Camera) {
    super('spotlight');
  }

  public update(cards: CardView[]): void {
    this.spotlightIds = cards.map((c) => c.id);
  }

  public layout(allActors: Map<string, CardActor>): void {
    if (this.spotlightIds.length === 0) return;

    // Anchor the spotlight in front of the camera so the showcased card is
    // always visible and facing the player, no matter where they orbit.
    const cam = this.camera;
    let center = new THREE.Vector3(0, this.floatHeight, 2.5);
    let rightDir = new THREE.Vector3(1, 0, 0);
    let faceQuat = new THREE.Quaternion();

    if (cam) {
      // Point on the camera's view axis, a fixed distance in front, then
      // lifted upward so the card hovers above the board instead of on it.
      const camPos = new THREE.Vector3();
      cam.getWorldPosition(camPos);
      const camDir = new THREE.Vector3();
      cam.getWorldDirection(camDir);
      center = camPos
        .clone()
        .add(camDir.clone().multiplyScalar(this.distanceInFront))
        .add(new THREE.Vector3(0, this.heightAboveCenter, 0));

      // Horizontal right vector so cards spread sideways in screen space.
      rightDir = new THREE.Vector3()
        .crossVectors(camDir, new THREE.Vector3(0, 1, 0))
        .normalize();

      // Rotation that makes the card's front face the camera.
      const lookTarget = center.clone().sub(camDir);
      const m = new THREE.Matrix4().lookAt(
        center,
        lookTarget,
        new THREE.Vector3(0, 1, 0),
      );
      faceQuat.setFromRotationMatrix(m);
    }

    // Stack items spread out horizontally with the top item centered
    const count = this.spotlightIds.length;
    const startOffset = -((count - 1) * this.cardSpacing) / 2;

    const faceRot = new THREE.Euler().setFromQuaternion(faceQuat);

    this.spotlightIds.forEach((id, index) => {
      const actor = allActors.get(id);
      if (!actor) return;

      actor.setZone('spotlight');
      actor.setFlatLayout(false);
      actor.setPileCount(1);

      const offset = startOffset + index * this.cardSpacing;
      const pos = center
        .clone()
        .add(rightDir.clone().multiplyScalar(offset));

      actor.setTarget(pos, faceRot, this.floatScale, 0.18);
    });
  }

  public getDropIndex(): number {
    return -1;
  }

  public getCardIds(): string[] {
    return this.spotlightIds;
  }

  public getActor(cardId: string): CardActor | undefined {
    return this.actors.get(cardId);
  }
}
