import * as THREE from 'three';
import { BaseZone } from './Zone';
import type { CardActor } from '../entities/CardActor';
import type { CardView } from '../types';

export class SelectionZone extends BaseZone {
  private poolIds: string[] = [];
  private topIds: string[] = [];
  private bottomIds: string[] = [];
  private cardW = 0.75;
  private cardH = 1.1;
  private spacing = 0.85; // cardW + gap
  private dropGroup = new THREE.Group();
  private dropTop: THREE.Mesh;
  private dropBottom: THREE.Mesh;
  private labelTop: THREE.Sprite;
  private labelBottom: THREE.Sprite;
  private scryLabel: THREE.Sprite;

  // Where this zone sits in the world
  public position = new THREE.Vector3(0, 0, 0);
  public rotation = new THREE.Euler(0, 0, 0);

  private static makeLabel(text: string, color: string): THREE.Sprite {
    const canvas = document.createElement('canvas');
    canvas.width = 256;
    canvas.height = 64;
    const ctx = canvas.getContext('2d')!;
    ctx.fillStyle = color;
    ctx.font = 'bold 40px monospace';
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    ctx.fillText(text, 128, 32);
    const texture = new THREE.CanvasTexture(canvas);
    const mat = new THREE.SpriteMaterial({
      map: texture,
      transparent: true,
      opacity: 0.7,
      blending: THREE.AdditiveBlending,
      depthWrite: false,
    });
    const sprite = new THREE.Sprite(mat);
    sprite.scale.set(1.0, 0.25, 1);
    return sprite;
  }

  private static makeScryLabel(): THREE.Sprite {
    const canvas = document.createElement('canvas');
    canvas.width = 512;
    canvas.height = 128;
    const ctx = canvas.getContext('2d')!;
    ctx.fillStyle = '#00e5ff';
    ctx.globalAlpha = 0.5;
    ctx.font = '900 96px monospace';
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    ctx.fillText('SCRY', 256, 64);
    const texture = new THREE.CanvasTexture(canvas);
    const mat = new THREE.SpriteMaterial({
      map: texture,
      transparent: true,
      opacity: 0.4,
      blending: THREE.AdditiveBlending,
      depthWrite: false,
    });
    const sprite = new THREE.Sprite(mat);
    sprite.scale.set(2.5, 0.6, 1);
    return sprite;
  }

  constructor(id: string) {
    super(id);
    const dropGeo = new THREE.PlaneGeometry(this.cardW * 1.1, this.cardH * 1.1);
    const dropMat = new THREE.MeshBasicMaterial({
      color: 0x00e5ff,
      transparent: true,
      opacity: 0.18,
      blending: THREE.AdditiveBlending,
      depthWrite: false,
      side: THREE.DoubleSide,
    });
    this.dropTop = new THREE.Mesh(dropGeo, dropMat.clone());
    this.dropBottom = new THREE.Mesh(dropGeo, dropMat.clone());
    this.dropTop.userData.scryDropZone = 'top';
    this.dropBottom.userData.scryDropZone = 'bottom';
    this.dropTop.visible = false;
    this.dropBottom.visible = false;

    this.labelTop = SelectionZone.makeLabel('TOP', '#00e5ff');
    this.labelTop.visible = false;
    this.labelBottom = SelectionZone.makeLabel('BOTTOM', '#00e5ff');
    this.labelBottom.visible = false;
    this.scryLabel = SelectionZone.makeScryLabel();
    this.scryLabel.visible = false;

    this.dropGroup.add(
      this.dropTop,
      this.dropBottom,
      this.labelTop,
      this.labelBottom,
      this.scryLabel,
    );
  }

  public setTransform(pos: THREE.Vector3, rot: THREE.Euler) {
    this.position.copy(pos);
    this.rotation.copy(rot);
  }

  public update(cards: CardView[]) {
    this.poolIds = cards.map((c) => c.id);
    this.topIds = [];
    this.bottomIds = [];
  }

  public setScryState(pool: CardView[], top: CardView[], bottom: CardView[]) {
    this.poolIds = pool.map((c) => c.id);
    this.topIds = top.map((c) => c.id);
    this.bottomIds = bottom.map((c) => c.id);
  }

  public getZoneForCard(cardId: string): 'pool' | 'top' | 'bottom' | null {
    if (this.poolIds.includes(cardId)) return 'pool';
    if (this.topIds.includes(cardId)) return 'top';
    if (this.bottomIds.includes(cardId)) return 'bottom';
    return null;
  }

  public getExtraInteractables(): THREE.Object3D[] {
    if (!this.dropGroup.visible) return [];
    return [this.dropTop, this.dropBottom];
  }

  public getSceneObjects(): THREE.Object3D[] {
    return [this.dropGroup];
  }

  public layout(allActors: Map<string, CardActor>) {
    const poolActors = this.poolIds
      .map((id) => allActors.get(id))
      .filter((a): a is CardActor => !!a);
    const topActors = this.topIds
      .map((id) => allActors.get(id))
      .filter((a): a is CardActor => !!a);
    const bottomActors = this.bottomIds
      .map((id) => allActors.get(id))
      .filter((a): a is CardActor => !!a);

    const totalCount =
      poolActors.length + topActors.length + bottomActors.length;
    if (totalCount === 0) {
      this.dropTop.visible = false;
      this.dropBottom.visible = false;
      this.labelTop.visible = false;
      this.labelBottom.visible = false;
      this.scryLabel.visible = false;
      return;
    }

    const rotMatrix = new THREE.Matrix4().makeRotationFromEuler(this.rotation);
    const targetRot = this.rotation.clone();
    targetRot.x -= Math.PI / 4;

    const placeRow = (
      actors: CardActor[],
      centerOffset: THREE.Vector3,
      stack: boolean,
    ) => {
      if (actors.length === 0) return;
      if (stack) {
        actors.forEach((actor, i) => {
          actor.setZone('hand');
          const offset = new THREE.Vector3(0, i * 0.002, -i * 0.03);
          offset.applyMatrix4(rotMatrix);
          const pos = this.position.clone().add(centerOffset).add(offset);

          const tiltedRot = targetRot.clone();
          tiltedRot.z += (i % 2 === 0 ? 1 : -1) * 0.01;

          actor.setTarget(pos, tiltedRot);
        });
      } else {
        const totalWidth = (actors.length - 1) * this.spacing;
        const startX = -totalWidth / 2;
        actors.forEach((actor, i) => {
          actor.setZone('hand');
          const offset = new THREE.Vector3(
            startX + i * this.spacing,
            i * 0.002,
            0,
          );
          offset.applyMatrix4(rotMatrix);
          const pos = this.position.clone().add(centerOffset).add(offset);

          const tiltedRot = targetRot.clone();
          tiltedRot.z += (i % 2 === 0 ? 1 : -1) * 0.01;

          actor.setTarget(pos, tiltedRot);
        });
      }
    };

    const poolCenter = new THREE.Vector3(0, 0, 0);
    const topCenter = new THREE.Vector3(-1.3, 0.2, 0);
    const bottomCenter = new THREE.Vector3(1.3, 0.2, 0);
    poolCenter.applyMatrix4(rotMatrix);
    topCenter.applyMatrix4(rotMatrix);
    bottomCenter.applyMatrix4(rotMatrix);

    placeRow(poolActors, poolCenter, false);
    placeRow(topActors, topCenter, true);
    placeRow(bottomActors, bottomCenter, true);

    this.dropTop.visible = true;
    this.dropBottom.visible = true;
    this.dropTop.position.copy(this.position.clone().add(topCenter));
    this.dropBottom.position.copy(this.position.clone().add(bottomCenter));
    this.dropTop.rotation.copy(targetRot);
    this.dropBottom.rotation.copy(targetRot);

    // Position "TOP" / "BOTTOM" labels just above the drop zones
    const labelOffset = new THREE.Vector3(0, 0.05, 0);
    this.labelTop.visible = true;
    this.labelTop.position.copy(this.dropTop.position).add(labelOffset);
    this.labelBottom.visible = true;
    this.labelBottom.position.copy(this.dropBottom.position).add(labelOffset);

    // "SCRY" etched label centered above the pool area
    const scryOffset = new THREE.Vector3(0, 1.8, 0);
    scryOffset.applyMatrix4(rotMatrix);
    this.scryLabel.visible = true;
    this.scryLabel.position.copy(
      this.position.clone().add(poolCenter).add(scryOffset),
    );
  }

  public setDropHover(zone: 'top' | 'bottom' | null) {
    const topHovered = zone === 'top';
    const bottomHovered = zone === 'bottom';
    (this.dropTop.material as THREE.MeshBasicMaterial).opacity = topHovered
      ? 0.35
      : 0.18;
    (this.dropBottom.material as THREE.MeshBasicMaterial).opacity = bottomHovered
      ? 0.35
      : 0.18;
    (this.labelTop.material as THREE.SpriteMaterial).opacity = topHovered
      ? 1.0
      : 0.7;
    (this.labelBottom.material as THREE.SpriteMaterial).opacity = bottomHovered
      ? 1.0
      : 0.7;
  }

  public getDropIndex(point: THREE.Vector3, viewerId: string | null): number {
    return 0; // TODO: Implement selection reordering
  }

  public getCardIds(viewerId: string | null): string[] {
    return [...this.poolIds, ...this.topIds, ...this.bottomIds];
  }
}
