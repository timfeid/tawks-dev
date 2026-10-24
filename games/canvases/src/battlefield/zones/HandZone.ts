import * as THREE from 'three';
import { BaseZone } from './Zone';
import type { CardActor } from '../entities/CardActor';
import type { CardView } from '../types';

export class HandZone extends BaseZone {
  private cardIds: string[] = [];
  private cardW = 0.75;
  private cardH = 1.1;
  private scale = 0.3;
  private isHovered = false;
  private handFocusFactor = 0;
  // Normalized pointer position [-1, 1] across the canvas. Drives a subtle
  // horizontal parallax on the hand tray so cards feel responsive to the
  // mouse and edge cards read as "reachable" without a full hover.
  private pointerX = 0;
  private pointerY = 0;
  private smoothedPointerX = 0;
  private smoothedPointerY = 0;

  constructor(private camera: THREE.Camera) {
    super('hand');
    // No handRoot attachment. We rely on ZoneManager putting actors in Scene.
  }

  public setHovered(hovered: boolean) {
    this.isHovered = hovered;
  }

  /** Normalized pointer coords in [-1, 1] (left→right, bottom→top). */
  public setPointer(x: number, y: number) {
    this.pointerX = THREE.MathUtils.clamp(x, -1, 1);
    this.pointerY = THREE.MathUtils.clamp(y, -1, 1);
  }

  public update(cards: CardView[]) {
    this.cardIds = cards.map((c) => c.id);
  }

  public tick(delta: number) {
    // Update focus factor for layout calculation
    const targetFactor = this.isHovered ? 1.0 : 0.0;
    this.handFocusFactor += (targetFactor - this.handFocusFactor) * 0.12;
    // Smooth the raw pointer into a slow-following value so the parallax
    // feels soft rather than jittery.
    this.smoothedPointerX +=
      (this.pointerX - this.smoothedPointerX) * 0.1;
    this.smoothedPointerY +=
      (this.pointerY - this.smoothedPointerY) * 0.1;
  }

  public layout(allActors: Map<string, CardActor>) {
    const actors = this.cardIds.map((id) => allActors.get(id)).filter((a): a is CardActor => !!a);

    if (actors.length === 0) return;
    actors.forEach((actor) => actor.setZone('hand'));

    // Calculate Smooth Factor from tick state
    const smoothFactor =
      this.handFocusFactor * this.handFocusFactor * (3 - 2 * this.handFocusFactor);

    // Layout Params
    const baseSpacing = this.cardW * 0.75;
    const spacing = actors.length > 8 ? (baseSpacing * 8) / actors.length : baseSpacing;
    const groupGap = this.cardW * 0.3;

    // Widths
    const playable = actors.filter((a) => (a as any).card.playable);
    const nonPlayable = actors.filter((a) => !(a as any).card.playable);
    const totalPlayableW = playable.length > 0 ? (playable.length - 1) * spacing * this.scale : 0;
    const totalNonPlayableW =
      nonPlayable.length > 0 ? (nonPlayable.length - 1) * spacing * this.scale : 0;
    const totalWidth =
      totalPlayableW +
      (playable.length > 0 && nonPlayable.length > 0 ? groupGap : 0) +
      totalNonPlayableW +
      this.cardW * this.scale;

    let currentX = -totalWidth / 2;

    // Position Logic (Interpolated Tray)
    const restingZ = -3.2;
    const activeZ = -1.6;

    const vFOV = THREE.MathUtils.degToRad((this.camera as THREE.PerspectiveCamera).fov);
    const screenHalfHeightAtFocus = Math.tan(vFOV / 2) * Math.abs(activeZ);
    const activeY = -screenHalfHeightAtFocus + (this.cardH * this.scale) / 2 + 0.05;
    const restingY = -1.25;

    const restingRotX = -1.2;
    const activeRotX = -0.3;

    const currentY = THREE.MathUtils.lerp(restingY, activeY, smoothFactor);
    const currentZ = THREE.MathUtils.lerp(restingZ, activeZ, smoothFactor);
    const currentRotX = THREE.MathUtils.lerp(restingRotX, activeRotX, smoothFactor);

    // Parallax offsets — subtle tray shift + tilt toward the pointer, but
    // only when the hand is open (smoothFactor > 0). Fully dormant while
    // the hand rests at the bottom of the screen.
    const parallaxX = -this.smoothedPointerX * 0.12 * smoothFactor;
    const parallaxYawTilt = this.smoothedPointerX * 0.025 * smoothFactor;
    const parallaxPitchTilt = this.smoothedPointerY * 0.02 * smoothFactor;

    // Camera Matrices (World Space Conversion)
    this.camera.updateMatrixWorld(); // Ensure up to date

    // Helper to place a group
    const placeGroup = (group: CardActor[], zOffset: number) => {
        group.forEach((actor, i) => {
            const x = currentX + (this.cardW * this.scale) / 2 + i * spacing * this.scale;

        // 1. Calculate Local Position on the Tray (relative to camera)
        const trayPos = new THREE.Vector3(parallaxX, currentY, currentZ);
            const cardOffset = new THREE.Vector3(x, 0, zOffset + i * 0.008);

        // Rotate offset by tray tilt (including the pointer-driven parallax tilt).
        const trayRot = new THREE.Euler(
          currentRotX + parallaxPitchTilt,
          parallaxYawTilt,
          0,
        );
        cardOffset.applyEuler(trayRot);

        const localPos = trayPos.add(cardOffset);

        // 2. Convert to World Space
        // Apply Camera Transform
        const worldPos = localPos.clone();
        worldPos.applyMatrix4(this.camera.matrixWorld);
        if (actor.isHoveredState()) {
          const towardCamera = this.camera.position
            .clone()
            .sub(worldPos)
            .normalize();
          worldPos.addScaledVector(towardCamera, 0.09);
        }

        // 3. Calculate World Rotation            // Camera Rotation + Tray Rotation
        const camQuat = this.camera.quaternion;
        const trayQuat = new THREE.Quaternion().setFromEuler(trayRot);
        const worldQuat = camQuat.clone().multiply(trayQuat);
        const worldRot = new THREE.Euler().setFromQuaternion(worldQuat);

        actor.setTarget(worldPos, worldRot, this.scale, 0.45);
      });
      if (group.length > 0) {
        currentX += (group.length - 1) * spacing * this.scale + this.cardW * this.scale;
      }
    };

    placeGroup(playable, 0);
    if (playable.length > 0 && nonPlayable.length > 0) currentX += groupGap;
    placeGroup(nonPlayable, -0.1);
  }

  public getDropIndex(point: THREE.Vector3, viewerId: string | null): number {
    return 0;
  }

  public getCardIds(viewerId: string | null): string[] {
    return this.cardIds;
  }
}
