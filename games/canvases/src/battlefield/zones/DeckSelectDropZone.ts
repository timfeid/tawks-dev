import * as THREE from 'three';
import { BaseZone } from './Zone';

type DropKind = 'as_is' | 'customize';

const createLabel = (text: string, color: string) => {
  const canvas = document.createElement('canvas');
  canvas.width = 512;
  canvas.height = 160;
  const ctx = canvas.getContext('2d');
  if (!ctx) return null;

  ctx.clearRect(0, 0, canvas.width, canvas.height);
  const baseGradient = ctx.createLinearGradient(0, 0, 0, canvas.height);
  baseGradient.addColorStop(0, 'rgba(18, 40, 60, 0.95)');
  baseGradient.addColorStop(0.5, 'rgba(8, 18, 28, 0.92)');
  baseGradient.addColorStop(1, 'rgba(6, 12, 18, 0.92)');
  ctx.fillStyle = baseGradient;
  ctx.strokeStyle = color;
  ctx.lineWidth = 10;
  ctx.lineJoin = 'round';

  ctx.beginPath();
  ctx.roundRect(16, 16, canvas.width - 32, canvas.height - 32, 28);
  ctx.shadowColor = color;
  ctx.shadowBlur = 16;
  ctx.fill();
  ctx.shadowBlur = 0;
  ctx.stroke();

  ctx.lineWidth = 3;
  ctx.strokeStyle = 'rgba(255,255,255,0.35)';
  ctx.stroke();

  ctx.fillStyle = '#f8fafc';
  ctx.font = '900 64px "Orbitron", sans-serif';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  ctx.shadowColor = 'rgba(0,0,0,0.6)';
  ctx.shadowBlur = 10;
  ctx.fillText(text.toUpperCase(), canvas.width / 2, canvas.height / 2 + 4);
  ctx.shadowBlur = 0;

  const texture = new THREE.CanvasTexture(canvas);
  texture.colorSpace = THREE.SRGBColorSpace;
  const material = new THREE.MeshBasicMaterial({
    map: texture,
    transparent: true,
    depthWrite: false,
    side: THREE.DoubleSide,
  });
  const mesh = new THREE.Mesh(new THREE.PlaneGeometry(2.4, 0.75), material);
  mesh.rotation.x = -Math.PI / 2;
  return mesh;
};

type DropVisual = {
  base: THREE.Mesh;
  ring: THREE.Mesh;
  label: THREE.Mesh | null;
  baseMat: THREE.MeshStandardMaterial;
  ringMat: THREE.MeshStandardMaterial;
  labelMat: THREE.MeshBasicMaterial | null;
  baseOpacity: number;
  ringOpacity: number;
  labelOpacity: number;
  hover: number;
  target: number;
};

export class DeckSelectDropZone extends BaseZone {
  private group = new THREE.Group();
  private drops: Record<DropKind, DropVisual>;
  private hoverKind: DropKind | null = null;

  constructor() {
    super('deck_select_drop');

    this.drops = {
      as_is: this.createDropVisual('As Is', 'as_is', 0x22c55e, -2.2),
      customize: this.createDropVisual('Customize', 'customize', 0x06b6d4, 2.2),
    };

    // Rotate to match camera orientation (cards/labels face the viewer)
    this.group.rotation.y = Math.PI / 2;
  }

  public update(_cards: any[]): void {}

  public layout(_allActors: Map<string, any>): void {}

  public tick(delta: number): void {
    (Object.keys(this.drops) as DropKind[]).forEach((kind) => {
      const drop = this.drops[kind];
      drop.hover = THREE.MathUtils.damp(drop.hover, drop.target, 8, delta);
      const hover = drop.hover;
      drop.baseMat.opacity = drop.baseOpacity + hover * 0.2;
      drop.baseMat.emissiveIntensity = 0.4 + hover * 0.6;
      drop.ringMat.opacity = drop.ringOpacity + hover * 0.3;
      drop.ringMat.emissiveIntensity = 0.9 + hover * 0.6;
      drop.ring.scale.setScalar(1 + hover * 0.08);
      if (drop.labelMat) {
        drop.labelMat.opacity = drop.labelOpacity + hover * 0.2;
      }
      if (drop.label) {
        drop.label.scale.setScalar(1 + hover * 0.04);
      }
    });
  }

  public getDropIndex(_point: THREE.Vector3, _viewerId: string | null): number {
    return 0;
  }

  public getCardIds(_viewerId: string | null): string[] {
    return [];
  }

  public getSceneObjects(): THREE.Object3D[] {
    return [this.group];
  }

  public getExtraInteractables(): THREE.Object3D[] {
    // Only expose the table drop targets while the zone is actually active
    // (i.e. the user is dragging a deck card over the table). Otherwise the
    // invisible circles would intercept clicks meant for the HUD buttons.
    if (!this.group.visible) return [];
    return [this.drops.as_is.base, this.drops.customize.base];
  }

  public setActive(active: boolean) {
    this.group.visible = active;
  }

  public setHover(kind: DropKind | null) {
    this.hoverKind = kind;
    (Object.keys(this.drops) as DropKind[]).forEach((key) => {
      this.drops[key].target = this.hoverKind === key ? 1 : 0;
    });
  }

  private createDropVisual(label: string, kind: DropKind, color: number, x: number): DropVisual {
    const circleGeo = new THREE.CircleGeometry(1.45, 48);
    const ringGeo = new THREE.RingGeometry(1.45, 1.72, 48);
    const baseMat = new THREE.MeshStandardMaterial({
      color,
      transparent: true,
      opacity: 0.18,
      side: THREE.DoubleSide,
      depthWrite: false,
      emissive: new THREE.Color(color),
      emissiveIntensity: 0.4,
      roughness: 0.4,
      metalness: 0.2,
    });
    const ringMat = new THREE.MeshStandardMaterial({
      color,
      transparent: true,
      opacity: 0.72,
      side: THREE.DoubleSide,
      depthWrite: false,
      emissive: new THREE.Color(color),
      emissiveIntensity: 0.9,
      roughness: 0.2,
      metalness: 0.15,
    });

    const base = new THREE.Mesh(circleGeo, baseMat);
    base.position.set(x, 0.02, 0);
    base.rotation.x = -Math.PI / 2;
    base.userData.deckSelectDrop = kind;
    this.group.add(base);

    const ring = new THREE.Mesh(ringGeo, ringMat);
    ring.position.set(x, 0.021, 0);
    ring.rotation.x = -Math.PI / 2;
    this.group.add(ring);

    const labelMesh = createLabel(label, `#${new THREE.Color(color).getHexString()}`);
    if (labelMesh) {
      labelMesh.position.set(x, 0.03, 0.9);
      this.group.add(labelMesh);
    }

    const labelMat =
      labelMesh && labelMesh.material instanceof THREE.MeshBasicMaterial
        ? labelMesh.material
        : null;

    return {
      base,
      ring,
      label: labelMesh,
      baseMat,
      ringMat,
      labelMat,
      baseOpacity: baseMat.opacity,
      ringOpacity: ringMat.opacity,
      labelOpacity: labelMat?.opacity ?? 1,
      hover: 0,
      target: 0,
    };
  }
}
