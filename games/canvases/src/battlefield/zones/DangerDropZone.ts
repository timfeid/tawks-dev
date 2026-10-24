import * as THREE from 'three';
import { BaseZone } from './Zone';

export class DangerDropZone extends BaseZone {
  private dropMesh: THREE.Mesh;
  private ringMesh: THREE.LineSegments;
  private particles: THREE.Points;
  private active = false;
  private hovered = false;

  constructor() {
    super('danger_drop');

    const radius = 2.4;
    const dropGeo = new THREE.CircleGeometry(radius, 64);
    const dropMat = new THREE.MeshBasicMaterial({
      color: 0xff3344,
      transparent: true,
      opacity: 0.06,
      side: THREE.DoubleSide,
      depthWrite: false,
      blending: THREE.AdditiveBlending,
    });
    this.dropMesh = new THREE.Mesh(dropGeo, dropMat);
    this.dropMesh.position.y = 0.02;
    this.dropMesh.rotation.x = -Math.PI / 2;
    this.dropMesh.userData.dangerDrop = true;

    // Etched Look
    const ringGeo = new THREE.RingGeometry(radius, radius + 0.08, 64);
    const ringMat = new THREE.MeshBasicMaterial({
      color: 0xff3344,
      transparent: true,
      opacity: 0.25,
      side: THREE.DoubleSide,
      depthWrite: false,
      blending: THREE.AdditiveBlending,
    });
    const ring = new THREE.Mesh(ringGeo, ringMat);
    ring.position.y = 0.021;
    ring.rotation.x = -Math.PI / 2;

    const ringEdges = new THREE.EdgesGeometry(ringGeo);
    const ringLineMat = new THREE.LineBasicMaterial({
      color: 0xff3344,
      transparent: true,
      opacity: 0.5,
      blending: THREE.AdditiveBlending,
    });
    this.ringMesh = new THREE.LineSegments(ringEdges, ringLineMat);
    this.ringMesh.position.copy(ring.position);
    this.ringMesh.rotation.copy(ring.rotation);

    // Particles flying up
    const partCount = 80;
    const posArr = new Float32Array(partCount * 3);
    const velArr: number[] = [];
    
    for (let i = 0; i < partCount; i++) {
      const angle = Math.random() * Math.PI * 2;
      const r = Math.random() * radius;
      posArr[i * 3] = Math.cos(angle) * r;
      posArr[i * 3 + 1] = Math.random() * 2.0;
      posArr[i * 3 + 2] = Math.sin(angle) * r;
      velArr.push(0.6 + Math.random() * 1.4);
    }

    const partGeo = new THREE.BufferGeometry();
    partGeo.setAttribute('position', new THREE.BufferAttribute(posArr, 3));
    const partMat = new THREE.PointsMaterial({
      color: 0xff3344,
      size: 0.08,
      transparent: true,
      opacity: 0.6,
      blending: THREE.AdditiveBlending,
      depthWrite: false,
    });
    this.particles = new THREE.Points(partGeo, partMat);
    this.particles.position.y = 0.05;
    this.particles.userData.velocities = velArr;

    this.setActive(false);
  }

  public tick(delta: number) {
    if (!this.active || !this.hovered) return;
    const posAttr = this.particles.geometry.attributes.position as THREE.BufferAttribute;
    const vels = this.particles.userData.velocities as number[];
    
    for (let i = 0; i < vels.length; i++) {
      let y = posAttr.getY(i);
      y += vels[i] * delta;
      if (y > 2.5) {
        y = 0;
        const angle = Math.random() * Math.PI * 2;
        const r = Math.random() * 2.4;
        posAttr.setX(i, Math.cos(angle) * r);
        posAttr.setZ(i, Math.sin(angle) * r);
      }
      posAttr.setY(i, y);
    }
    posAttr.needsUpdate = true;
  }

  public getSceneObjects(): THREE.Object3D[] {
    return [this.dropMesh, this.ringMesh, this.particles];
  }

  public setPosition(x: number, z: number) {
    this.dropMesh.position.x = x;
    this.dropMesh.position.z = z;
    this.ringMesh.position.x = x;
    this.ringMesh.position.z = z;
    this.particles.position.x = x;
    this.particles.position.z = z;
  }

  public setActive(active: boolean) {
    this.active = active;
    this.dropMesh.visible = active;
    this.ringMesh.visible = active;
    // Particles visibility handled by hover
    this.particles.visible = active && this.hovered;
    if (!active) this.setHovered(false);
  }

  public setHovered(hovered: boolean) {
    this.hovered = hovered;
    if (this.active) {
      this.particles.visible = hovered;
      (this.dropMesh.material as THREE.MeshBasicMaterial).opacity = hovered ? 0.15 : 0.06;
      (this.ringMesh.material as THREE.LineBasicMaterial).opacity = hovered ? 0.9 : 0.5;
    }
  }

  public getExtraInteractables(): THREE.Object3D[] {
    // Skip when hidden — `visible: false` doesn't stop raycasts, and the
    // invisible drop mesh would otherwise intercept clicks meant for the HUD.
    if (!this.dropMesh.visible) return [];
    return [this.dropMesh];
  }

  public update(_cards: any[]): void {}
  public layout(_allActors: Map<string, any>): void {}
  public getDropIndex(_point: THREE.Vector3, _viewerId: string | null): number { return 0; }
  public getCardIds(_viewerId: string | null): string[] { return []; }
}
