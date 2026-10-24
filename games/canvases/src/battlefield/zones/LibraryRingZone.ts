import * as THREE from 'three';
import { BaseZone } from './Zone';
import type { CardActor } from '../entities/CardActor';
import type { CardView } from '../types';

export class LibraryRingZone extends BaseZone {
  private cardIds: string[] = [];
  private cardW = 0.75;
  private cardH = 1.1;
  private radius = 10;
  private selectedRadius = 9;
  private scale = 0.75;
  private selectedScale = 1.0;
  private selectedLift = 0.2;
  private selectedId: string | null = null;
  private startAngle = Math.PI * 0.5;
  private totalArc = Math.PI * 2;
  private rotationOffset = 0;
  private pileCount = 0;
  private pileSelectedCount = 0;
  private layoutMode: 'ring' | 'line' | 'grid' = 'ring';
  private gridColumns = 10;
  private lineSpacing = 1.2;
  private lineZ = 0;
  private lineY = 0.04;
  private flat = false;
  private lineAxis: 'x' | 'z' = 'x';
  private lineOffset = 0;
  private lineCross = 0;
  private lineDepthLift = 0;
  private lineRotation = 0;
  private lineScaleByDistance: number[] | null = null;
  private rows = 1;
  private centerSelected = false;
  // Bow the line outward in the cross axis by distance from the selected card.
  // `lineCurveDepth` = max offset (world units) applied at the line's ends.
  private lineCurveDepth = 0;

  // Visuals
  private trackMesh: THREE.Line | null = null;
  private showTrack = false;

  constructor(id: string = 'library_ring') {
    super(id);
  }

  public getSceneObjects(): THREE.Object3D[] {
    return this.trackMesh ? [this.trackMesh] : [];
  }

  public setShowTrack(show: boolean) {
    this.showTrack = show;
    this.updateTrack();
  }

  private updateTrack() {
    if (!this.showTrack) {
      if (this.trackMesh) {
        this.trackMesh.visible = false;
      }
      return;
    }

    if (this.layoutMode === 'line') {
      if (this.trackMesh) this.trackMesh.visible = false;
      return;
    }

    const segments = 128;
    const points = [];
    const arc = this.totalArc;
    const start = this.startAngle;
    
    for (let i = 0; i <= segments; i++) {
      const theta = start + (i / segments) * arc;
      points.push(new THREE.Vector3(
        Math.cos(theta) * this.radius,
        this.lineY + 0.01,
        Math.sin(theta) * this.radius
      ));
    }

    const geometry = new THREE.BufferGeometry().setFromPoints(points);
    if (!this.trackMesh) {
      const material = new THREE.LineBasicMaterial({
        color: 0x00fbff,
        transparent: true,
        opacity: 0.6,
        blending: THREE.AdditiveBlending,
        linewidth: 2,
      });
      this.trackMesh = new THREE.Line(geometry, material);
      this.trackMesh.renderOrder = 4;
    } else {
      this.trackMesh.geometry.dispose();
      this.trackMesh.geometry = geometry;
      this.trackMesh.visible = true;
    }
  }

  public update(cards: CardView[], selectedId?: string | null) {
    this.cardIds = cards.map((c) => c.id);
    this.selectedId = selectedId ?? null;
  }

  public setLayoutConfig(config: {
    radius?: number;
    selectedRadius?: number;
    scale?: number;
    selectedScale?: number;
    selectedLift?: number;
    startAngle?: number;
    totalArc?: number;
    rotationOffset?: number;
    pileCount?: number;
    pileSelectedCount?: number;
    layoutMode?: 'ring' | 'line' | 'grid';
    gridColumns?: number;
    lineSpacing?: number;
    lineZ?: number;
    lineY?: number;
    flat?: boolean;
    lineAxis?: 'x' | 'z';
    lineOffset?: number;
    lineCross?: number;
    lineDepthLift?: number;
    lineRotation?: number;
    lineScaleByDistance?: number[] | null;
    rows?: number;
    centerSelected?: boolean;
    lineCurveDepth?: number;
  }) {
    if (typeof config.radius === 'number') this.radius = config.radius;
    if (typeof config.selectedRadius === 'number') this.selectedRadius = config.selectedRadius;
    if (typeof config.scale === 'number') this.scale = config.scale;
    if (typeof config.selectedScale === 'number') this.selectedScale = config.selectedScale;
    if (typeof config.selectedLift === 'number') this.selectedLift = config.selectedLift;
    if (typeof config.startAngle === 'number') this.startAngle = config.startAngle;
    if (typeof config.totalArc === 'number') this.totalArc = config.totalArc;
    if (typeof config.rotationOffset === 'number') this.rotationOffset = config.rotationOffset;
    if (typeof config.pileCount === 'number') this.pileCount = config.pileCount;
    if (typeof config.pileSelectedCount === 'number') this.pileSelectedCount = config.pileSelectedCount;
    if (config.layoutMode) this.layoutMode = config.layoutMode;
    if (typeof config.gridColumns === 'number') this.gridColumns = config.gridColumns;
    if (typeof config.lineSpacing === 'number') this.lineSpacing = config.lineSpacing;
    if (typeof config.lineZ === 'number') this.lineZ = config.lineZ;
    if (typeof config.lineY === 'number') this.lineY = config.lineY;
    if (typeof config.flat === 'boolean') this.flat = config.flat;
    if (config.lineAxis) this.lineAxis = config.lineAxis;
    if (typeof config.lineOffset === 'number') this.lineOffset = config.lineOffset;
    if (typeof config.lineCross === 'number') this.lineCross = config.lineCross;
    if (typeof config.lineDepthLift === 'number') this.lineDepthLift = config.lineDepthLift;
    if (typeof config.lineRotation === 'number') this.lineRotation = config.lineRotation;
    if (typeof config.rows === 'number') this.rows = config.rows;
    if (typeof config.centerSelected === 'boolean') this.centerSelected = config.centerSelected;
    if (typeof config.lineCurveDepth === 'number') this.lineCurveDepth = config.lineCurveDepth;
    if (config.lineScaleByDistance !== undefined) {
      this.lineScaleByDistance = config.lineScaleByDistance;
    }
    this.updateTrack();
  }

  public layout(allActors: Map<string, CardActor>) {
    const actors = this.cardIds
      .map((id) => allActors.get(id))
      .filter((a): a is CardActor => !!a);
    if (actors.length === 0) return;

    const count = actors.length;

    const selectedIndex =
      this.selectedId && this.cardIds.includes(this.selectedId)
        ? this.cardIds.indexOf(this.selectedId)
        : Math.floor((count - 1) / 2);

    if (this.layoutMode === 'grid') {
      const cols = this.gridColumns;
      const colSpacing = this.lineSpacing;
      const rowSpacing = colSpacing * 1.45;
      const rows = Math.ceil(count / cols);
      const gridWidth = (cols - 1) * colSpacing;
      const gridDepth = (rows - 1) * rowSpacing;

      actors.forEach((actor, index) => {
        actor.setZone('hand');
        actor.setFlatLayout(true);
        const col = index % cols;
        const row = Math.floor(index / cols);
        const x = col * colSpacing - gridWidth / 2;
        const z = row * rowSpacing - gridDepth / 2;
        const yOffset = index * 0.002;
        const pos = new THREE.Vector3(x, this.lineY + yOffset, z);
        const rot = new THREE.Euler(-Math.PI / 2, 0, Math.PI / 2);
        actor.setTarget(pos, rot, this.scale, 0.15);
      });
      return;
    }

    if (this.layoutMode === 'line') {
      const spacing = this.lineSpacing;
      // When `centerSelected` is on, anchor the line so the selected card sits
      // at axisPos = 0 (ring's horizontal center) regardless of its index.
      const centerIndex = this.centerSelected ? selectedIndex : (count - 1) / 2;
      const start = -centerIndex * spacing;
      const mid = (count - 1) / 2;
      actors.forEach((actor, index) => {
        actor.setZone('hand');
        actor.setFlatLayout(this.flat);
        const dist = Math.abs(index - selectedIndex);
        const isSelected = dist === 0;
        let scale = isSelected ? this.selectedScale : this.scale;
        if (this.lineScaleByDistance && this.lineScaleByDistance.length > 0) {
          // Clamp past the array's length so far-out cards don't "pop" back
          // up to the base scale; they stay at the tail ratio.
          const idx = Math.min(dist, this.lineScaleByDistance.length - 1);
          scale = this.selectedScale * this.lineScaleByDistance[idx];
        }
        const depthNorm = mid > 0 ? Math.abs(index - mid) / mid : 0;
        const depthLift = this.lineDepthLift * (1 - depthNorm);
        const lift = depthLift + (isSelected ? this.selectedLift : 0);
        const axisPos = this.lineOffset + start + index * spacing;
        // Quadratic curve: bow the line outward along the cross axis based on
        // distance from the selected card. Endpoints get pushed the full
        // `lineCurveDepth`; the selected card stays on-axis (offset = 0).
        const maxDist = Math.max(
          selectedIndex,
          count - 1 - selectedIndex,
          1,
        );
        const distNorm = Math.abs(index - selectedIndex) / maxDist;
        const curveOffset = this.lineCurveDepth * distNorm * distNorm;
        const crossPos = this.lineCross + curveOffset;
        const x = this.lineAxis === 'x' ? axisPos : crossPos;
        const z = this.lineAxis === 'z' ? axisPos : crossPos;

        // Add tiny Y offset to prevent Z-fighting and provide depth
        const yOffset = index * 0.002;

        const pos = new THREE.Vector3(
          x,
          this.lineY + lift + yOffset,
          z,
        );
        // Yaw neighbors toward the selected card so the curve reads as a
        // fan, not a straight row. Endpoints pick up the most rotation.
        const curveYawRange = this.lineCurveDepth > 0 ? 0.45 : 0;
        const curveSign = index < selectedIndex ? -1 : index > selectedIndex ? 1 : 0;
        const curveYaw = curveYawRange * distNorm * curveSign;
        const rot = this.flat
          ? new THREE.Euler(
              -Math.PI / 2 + (index % 2 === 0 ? 0.01 : -0.01),
              0,
              this.lineRotation + Math.PI / 2 + curveYaw + (index % 2 === 0 ? 0.005 : -0.005)
            )
          : new THREE.Euler(0, this.lineRotation + curveYaw, 0);
        actor.setTarget(pos, rot, scale, isSelected ? 0.12 : 0.2);
        const pCount = isSelected ? this.pileSelectedCount : this.pileCount;
        if (pCount >= 0) {
          actor.setPileCount(pCount);
        }
      });
      return;
    }

    const arc = this.totalArc;
    const angleStep = count > 1 && arc < Math.PI * 2 ? arc / (count - 1) : arc / count;
    const startAngle = this.startAngle;

    actors.forEach((actor, index) => {
      actor.setZone('hand');
      actor.setFlatLayout(this.flat);
      const angle = startAngle + index * angleStep;
      const isSelected = index === selectedIndex;
      
      // Multi-row logic: alternate radius and lift for cards
      let radius = isSelected ? this.selectedRadius : this.radius;
      let rowLift = 0;
      if (!isSelected && this.rows > 1) {
        const rowIndex = index % this.rows;
        // Pushes alternate rows further out and slightly up
        radius += rowIndex * 0.8; 
        rowLift = rowIndex * 0.04;
      }

      const scale = isSelected ? this.selectedScale : this.scale;
      const lift = isSelected ? this.selectedLift : 0;

      // Add tiny Y offset to prevent Z-fighting and provide depth
      const yOffset = index * 0.002;

      const pos = new THREE.Vector3(
        Math.cos(angle) * radius,
        (this.flat ? this.lineY : this.cardH * 0.5 + 0.12) + lift + rowLift + yOffset,
        Math.sin(angle) * radius,
      );
      const rot = this.flat
        ? new THREE.Euler(
            -Math.PI / 2 + (index % 2 === 0 ? 0.01 : -0.01),
            0,
            -angle + this.rotationOffset + Math.PI / 2 + (index % 2 === 0 ? 0.005 : -0.005),
          )
        : new THREE.Euler(0, -angle - Math.PI / 2 + this.rotationOffset, 0);
      actor.setTarget(pos, rot, scale, isSelected ? 0.12 : 0.2);
      const pCount = isSelected ? this.pileSelectedCount : this.pileCount;
      if (pCount >= 0) {
        actor.setPileCount(pCount);
      }
    });
  }

  public getDropIndex(point: THREE.Vector3, viewerId: string | null): number {
    return 0;
  }

  public getCardIds(viewerId: string | null): string[] {
    return this.cardIds;
  }
}