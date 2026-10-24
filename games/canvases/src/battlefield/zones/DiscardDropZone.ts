import * as THREE from 'three';
import { BaseZone } from './Zone';

export class DiscardDropZone extends BaseZone {
  private dropMesh: THREE.Mesh;
  private ringMesh: THREE.LineSegments;
  private particles: THREE.Points;
  private particles2: THREE.Points; // Second particle layer for intensity
  private labelSprite: THREE.Sprite;
  private active = false;
  private hovered = false;

  constructor() {
    super('discard_drop');

    const radius = 2.0;
    const dropGeo = new THREE.CircleGeometry(radius, 64);
    const dropMat = new THREE.MeshBasicMaterial({
      color: 0x9333ea, // Purple for discard
      transparent: true,
      opacity: 0.08,
      side: THREE.DoubleSide,
      depthWrite: false,
      blending: THREE.AdditiveBlending,
    });
    this.dropMesh = new THREE.Mesh(dropGeo, dropMat);
    this.dropMesh.position.y = 0.02;
    this.dropMesh.rotation.x = -Math.PI / 2;
    this.dropMesh.userData.discardDrop = true;

    // Etched Ring
    const ringGeo = new THREE.RingGeometry(radius, radius + 0.08, 64);
    const ringMat = new THREE.MeshBasicMaterial({
      color: 0x9333ea,
      transparent: true,
      opacity: 0.3,
      side: THREE.DoubleSide,
      depthWrite: false,
      blending: THREE.AdditiveBlending,
    });
    const ring = new THREE.Mesh(ringGeo, ringMat);
    ring.position.y = 0.021;
    ring.rotation.x = -Math.PI / 2;

    const ringEdges = new THREE.EdgesGeometry(ringGeo);
    const ringLineMat = new THREE.LineBasicMaterial({
      color: 0x9333ea,
      transparent: true,
      opacity: 0.6,
      blending: THREE.AdditiveBlending,
    });
    this.ringMesh = new THREE.LineSegments(ringEdges, ringLineMat);
    this.ringMesh.position.copy(ring.position);
    this.ringMesh.rotation.copy(ring.rotation);

    // Falling particles (discard goes down) - Layer 1: Fast, small particles
    const partCount = 150;
    const posArr = new Float32Array(partCount * 3);
    const velArr: number[] = [];

    for (let i = 0; i < partCount; i++) {
      const angle = Math.random() * Math.PI * 2;
      const r = Math.random() * radius;
      posArr[i * 3] = Math.cos(angle) * r;
      posArr[i * 3 + 1] = Math.random() * 2.0;
      posArr[i * 3 + 2] = Math.sin(angle) * r;
      velArr.push(-(0.6 + Math.random() * 1.2)); // Faster falling
    }

    const partGeo = new THREE.BufferGeometry();
    partGeo.setAttribute('position', new THREE.BufferAttribute(posArr, 3));
    const partMat = new THREE.PointsMaterial({
      color: 0x9333ea,
      size: 0.08,
      transparent: true,
      opacity: 0.6,
      blending: THREE.AdditiveBlending,
      depthWrite: false,
    });
    this.particles = new THREE.Points(partGeo, partMat);
    this.particles.position.y = 0.05;
    this.particles.userData.velocities = velArr;

    // Layer 2: Slower, larger particles for depth
    const partCount2 = 80;
    const posArr2 = new Float32Array(partCount2 * 3);
    const velArr2: number[] = [];

    for (let i = 0; i < partCount2; i++) {
      const angle = Math.random() * Math.PI * 2;
      const r = Math.random() * radius * 1.2;
      posArr2[i * 3] = Math.cos(angle) * r;
      posArr2[i * 3 + 1] = Math.random() * 2.5;
      posArr2[i * 3 + 2] = Math.sin(angle) * r;
      velArr2.push(-(0.3 + Math.random() * 0.5)); // Slower
    }

    const partGeo2 = new THREE.BufferGeometry();
    partGeo2.setAttribute('position', new THREE.BufferAttribute(posArr2, 3));
    const partMat2 = new THREE.PointsMaterial({
      color: 0xb366ff, // Lighter purple
      size: 0.12,
      transparent: true,
      opacity: 0.4,
      blending: THREE.AdditiveBlending,
      depthWrite: false,
    });
    this.particles2 = new THREE.Points(partGeo2, partMat2);
    this.particles2.position.y = 0.05;
    this.particles2.userData.velocities = velArr2;

    // Text label "DISCARD"
    const canvas = document.createElement('canvas');
    canvas.width = 512;
    canvas.height = 128;
    const ctx = canvas.getContext('2d')!;
    ctx.fillStyle = '#9333ea';
    ctx.font = 'bold 72px monospace';
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    ctx.fillText('DISCARD', 256, 64);

    const texture = new THREE.CanvasTexture(canvas);
    const spriteMat = new THREE.SpriteMaterial({
      map: texture,
      transparent: true,
      opacity: 0.7,
      blending: THREE.AdditiveBlending,
      depthWrite: false,
    });
    this.labelSprite = new THREE.Sprite(spriteMat);
    this.labelSprite.scale.set(2.0, 0.5, 1);
    this.labelSprite.position.y = 0.1;

    this.setActive(false);
  }

  public tick(delta: number) {
    if (!this.active || !this.hovered) return;

    // Animate first particle layer
    const posAttr = this.particles.geometry.attributes.position as THREE.BufferAttribute;
    const vels = this.particles.userData.velocities as number[];

    for (let i = 0; i < vels.length; i++) {
      let y = posAttr.getY(i);
      y += vels[i] * delta;
      if (y < 0) {
        y = 2.0;
        const angle = Math.random() * Math.PI * 2;
        const r = Math.random() * 2.0;
        posAttr.setX(i, Math.cos(angle) * r);
        posAttr.setZ(i, Math.sin(angle) * r);
      }
      posAttr.setY(i, y);
    }
    posAttr.needsUpdate = true;

    // Animate second particle layer
    const posAttr2 = this.particles2.geometry.attributes.position as THREE.BufferAttribute;
    const vels2 = this.particles2.userData.velocities as number[];

    for (let i = 0; i < vels2.length; i++) {
      let y = posAttr2.getY(i);
      y += vels2[i] * delta;
      if (y < 0) {
        y = 2.5;
        const angle = Math.random() * Math.PI * 2;
        const r = Math.random() * 2.4;
        posAttr2.setX(i, Math.cos(angle) * r);
        posAttr2.setZ(i, Math.sin(angle) * r);
      }
      posAttr2.setY(i, y);
    }
    posAttr2.needsUpdate = true;
  }

  public getSceneObjects(): THREE.Object3D[] {
    return [this.dropMesh, this.ringMesh, this.particles, this.particles2, this.labelSprite];
  }

  public setPosition(x: number, z: number) {
    this.dropMesh.position.x = x;
    this.dropMesh.position.z = z;
    this.ringMesh.position.x = x;
    this.ringMesh.position.z = z;
    this.particles.position.x = x;
    this.particles.position.z = z;
    this.particles2.position.x = x;
    this.particles2.position.z = z;
    this.labelSprite.position.x = x;
    this.labelSprite.position.z = z;
  }

  public setActive(active: boolean) {
    this.active = active;
    this.dropMesh.visible = active;
    this.ringMesh.visible = active;
    this.labelSprite.visible = active;
    this.particles.visible = active && this.hovered;
    this.particles2.visible = active && this.hovered;
    if (!active) this.setHovered(false);
  }

  public setHovered(hovered: boolean) {
    this.hovered = hovered;
    if (this.active) {
      this.particles.visible = hovered;
      this.particles2.visible = hovered;
      (this.dropMesh.material as THREE.MeshBasicMaterial).opacity = hovered ? 0.18 : 0.08;
      (this.ringMesh.material as THREE.LineBasicMaterial).opacity = hovered ? 0.95 : 0.6;
      (this.labelSprite.material as THREE.SpriteMaterial).opacity = hovered ? 0.9 : 0.7;
    }
  }

  public getExtraInteractables(): THREE.Object3D[] {
    if (!this.dropMesh.visible) return [];
    return [this.dropMesh];
  }

  public update(_cards: any[]): void {}
  public layout(_allActors: Map<string, any>): void {}
  public getDropIndex(_point: THREE.Vector3, _viewerId: string | null): number { return 0; }
  public getCardIds(_viewerId: string | null): string[] { return []; }
}
