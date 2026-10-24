import * as THREE from 'three';
import type { CardFactory } from '../entities/CardFactory';

// ─── Types ───────────────────────────────────────────────────────────────────

export type HudAnchor =
  | 'top-left'
  | 'top-center'
  | 'top-right'
  | 'bottom-left'
  | 'bottom-center'
  | 'bottom-right'
  | 'center';

export interface HudElement {
  id: string;
  type: 'button' | 'text-button' | 'text-button-icon' | 'label' | 'seat-card';
  label: string;
  actionId: string;
  color: string;
  active?: boolean;
  anchor: HudAnchor;
  offsetX?: number;
  offsetY?: number;
  options?: Record<string, any>;
  payload?: Record<string, any>;
  interactive?: boolean;
  // text-button-icon
  iconText?: string;
  // seat-card
  subtitle?: string;
  ready?: boolean;
}

export interface HudBar {
  id: string;
  anchor: HudAnchor;
  width: 'full' | number;  // 'full' stretches to screen width, number = fixed world units
  height: number;           // height in world units
  offsetX?: number;
  offsetY?: number;
  color?: string;           // border/accent color
  opacity?: number;         // background opacity (default 0.85)
}

export interface HudConfig {
  bars?: HudBar[];
  elements: HudElement[];
}

// ─── Constants ───────────────────────────────────────────────────────────────

const HUD_DISTANCE = 8;
const MARGIN = 0.15;
const BAR_EDGE_MARGIN = 0; // Bars flush to screen edge
const DEG2RAD = Math.PI / 180;
const HUD_SCALE = 0.4; // Scale down elements for HUD context

// ─── HudManager ──────────────────────────────────────────────────────────────

export class HudManager {
  private hudGroup: THREE.Group;
  private meshes: THREE.Object3D[] = [];
  private barMeshes: THREE.Mesh[] = [];
  private config: HudConfig = { elements: [] };
  private configKey = '';
  private camera: THREE.PerspectiveCamera;
  private factory: CardFactory;
  private lastAspect = 0;

  constructor(camera: THREE.PerspectiveCamera, factory: CardFactory) {
    this.camera = camera;
    this.factory = factory;
    this.hudGroup = new THREE.Group();
    this.hudGroup.renderOrder = 100;
    camera.add(this.hudGroup);
  }

  // ─── Public API ────────────────────────────────────────────────────────────

  public setConfig(config: HudConfig): void {
    const key = JSON.stringify(config);
    if (key === this.configKey) return;
    this.configKey = key;
    this.config = config;
    this.rebuild();
  }

  public clear(): void {
    this.config = { elements: [] };
    this.configKey = '';
    this.clearMeshes();
  }

  public update(aspect: number): void {
    if (Math.abs(aspect - this.lastAspect) > 0.01) {
      this.lastAspect = aspect;
      this.repositionAll();
    }

    // Hover animations
    for (const mesh of this.meshes) {
      if (!mesh.userData.isButton) continue;
      const base = mesh.userData.baseScale ?? 1.0;
      const target = mesh.userData.isHovered ? base * 1.1 : base;
      mesh.scale.x = THREE.MathUtils.lerp(mesh.scale.x, target, 0.15);
      mesh.scale.y = THREE.MathUtils.lerp(mesh.scale.y, target, 0.15);
    }
  }

  public getInteractables(): THREE.Object3D[] {
    return this.meshes.filter((m) => m.userData.isButton);
  }

  public dispose(): void {
    this.clearMeshes();
    this.camera.remove(this.hudGroup);
  }

  // ─── Internal ──────────────────────────────────────────────────────────────

  private rebuild(): void {
    this.clearMeshes();

    // Build bars first (backgrounds)
    if (this.config.bars) {
      for (const bar of this.config.bars) {
        const barMesh = this.createBarMesh(bar);
        if (barMesh) {
          barMesh.userData.hudAnchor = bar.anchor;
          barMesh.userData.hudOffsetX = bar.offsetX ?? 0;
          barMesh.userData.hudOffsetY = bar.offsetY ?? 0;
          barMesh.userData.hudBarId = bar.id;
          barMesh.userData.hudBarWidth = bar.width;
          barMesh.userData.hudBarHeight = bar.height;
          this.hudGroup.add(barMesh);
          this.barMeshes.push(barMesh);
        }
      }
    }

    // Build elements
    const utils = this.factory.getTextureUtils();

    for (const el of this.config.elements) {
      let mesh: THREE.Object3D | null = null;

      switch (el.type) {
        case 'button':
          mesh = utils.create3DButton(el.label, el.actionId, el.color, el.options);
          break;
        case 'text-button':
          mesh = utils.create3DTextButton(
            el.label,
            el.actionId,
            el.color,
            el.active ?? false,
            el.options,
          );
          break;
        case 'text-button-icon':
          mesh = utils.create3DTextButtonWithIcon(
            el.label,
            el.iconText ?? '',
            el.actionId,
            el.color,
            el.active ?? false,
            el.options,
          );
          break;
        case 'label':
          mesh = utils.create3DLabel(el.label, el.color);
          break;
        case 'seat-card':
          mesh = utils.createPlayerSeatCard(
            el.label,
            el.subtitle ?? null,
            el.ready ?? false,
            { color: el.color },
          );
          break;
      }

      if (!mesh) continue;

      mesh.userData.hudAnchor = el.anchor;
      mesh.userData.hudOffsetX = el.offsetX ?? 0;
      mesh.userData.hudOffsetY = el.offsetY ?? 0;
      mesh.userData.hudId = el.id;

      if (el.payload) mesh.userData.payload = el.payload;
      if (el.interactive === false) mesh.userData.isButton = false;

      // Scale down for HUD context
      mesh.scale.multiplyScalar(HUD_SCALE);
      mesh.userData.baseScale = HUD_SCALE;

      // HUD render settings — always on top, no depth fighting
      mesh.renderOrder = 200;
      mesh.traverse((child) => {
        if (child instanceof THREE.Mesh) {
          child.material.depthTest = false;
          child.material.depthWrite = false;
          child.renderOrder = 200;
        }
      });

      this.hudGroup.add(mesh);
      this.meshes.push(mesh);
    }

    this.repositionAll();
  }

  private createBarMesh(bar: HudBar): THREE.Mesh | null {
    const color = bar.color ?? '#22d3ee';
    const opacity = bar.opacity ?? 1.0;
    const barH = bar.height;
    // For 'full' width bars, we'll use a large initial width and resize in repositionAll
    const barW = bar.width === 'full' ? 20 : bar.width;

    const pixelRatio = 2;
    const logicalW = 512;
    const logicalH = Math.max(1, Math.round(logicalW * (barH / barW)));
    const canvas = document.createElement('canvas');
    canvas.width = Math.max(1, Math.round(logicalW * pixelRatio));
    canvas.height = Math.max(1, Math.round(logicalH * pixelRatio));
    const ctx = canvas.getContext('2d');
    if (!ctx) return null;
    ctx.scale(pixelRatio, pixelRatio);
    ctx.clearRect(0, 0, logicalW, logicalH);

    // Dark opaque background
    const radius = 8;
    const r = 2, g = 6, b = 14;
    ctx.fillStyle = opacity >= 1 ? `rgb(${r}, ${g}, ${b})` : `rgba(${r}, ${g}, ${b}, ${opacity})`;
    ctx.beginPath();
    ctx.moveTo(radius, 0);
    ctx.lineTo(logicalW - radius, 0);
    ctx.quadraticCurveTo(logicalW, 0, logicalW, radius);
    ctx.lineTo(logicalW, logicalH - radius);
    ctx.quadraticCurveTo(logicalW, logicalH, logicalW - radius, logicalH);
    ctx.lineTo(radius, logicalH);
    ctx.quadraticCurveTo(0, logicalH, 0, logicalH - radius);
    ctx.lineTo(0, radius);
    ctx.quadraticCurveTo(0, 0, radius, 0);
    ctx.closePath();
    ctx.fill();

    // Subtle border
    ctx.strokeStyle = color;
    ctx.lineWidth = 1.5;
    ctx.globalAlpha = 0.4;
    ctx.stroke();
    ctx.globalAlpha = 1;

    // Bottom edge glow line
    ctx.strokeStyle = color;
    ctx.lineWidth = 1;
    ctx.globalAlpha = 0.25;
    ctx.beginPath();
    ctx.moveTo(radius, logicalH - 1);
    ctx.lineTo(logicalW - radius, logicalH - 1);
    ctx.stroke();
    ctx.globalAlpha = 1;

    const texture = new THREE.CanvasTexture(canvas);
    texture.colorSpace = THREE.SRGBColorSpace;
    texture.generateMipmaps = false;
    texture.minFilter = THREE.LinearFilter;
    texture.magFilter = THREE.LinearFilter;

    const geometry = new THREE.PlaneGeometry(barW, barH);
    const material = new THREE.MeshBasicMaterial({
      map: texture,
      transparent: true,
      depthTest: false,
      depthWrite: false,
      side: THREE.DoubleSide,
    });

    const mesh = new THREE.Mesh(geometry, material);
    mesh.renderOrder = 190; // Behind elements (200) but in front of scene
    mesh.userData.isButton = false;
    return mesh;
  }

  private repositionAll(): void {
    const fov = this.camera.fov;
    const aspect = this.lastAspect || 1;
    const halfH = HUD_DISTANCE * Math.tan((fov / 2) * DEG2RAD);
    const halfW = halfH * aspect;

    // Position bars — flush to screen edges
    for (const bar of this.barMeshes) {
      const anchor: HudAnchor = bar.userData.hudAnchor;
      const ox = bar.userData.hudOffsetX ?? 0;
      const oy = bar.userData.hudOffsetY ?? 0;
      const barH = bar.userData.hudBarHeight;

      // Scale full-width bars to screen width
      if (bar.userData.hudBarWidth === 'full') {
        const fullW = halfW * 2 + 0.1; // Slight overshoot to avoid edge gaps
        bar.geometry.dispose();
        bar.geometry = new THREE.PlaneGeometry(fullW, barH);
      }

      // Position flush to edge — bar center offset by half its height
      const { x, y } = this.anchorPosition(anchor, halfW, halfH, BAR_EDGE_MARGIN);
      // Adjust so bar bottom/top is exactly at screen edge
      let adjustedY = y;
      if (anchor.startsWith('top')) {
        adjustedY = halfH - barH / 2; // Top edge flush
      } else if (anchor.startsWith('bottom')) {
        adjustedY = -halfH + barH / 2; // Bottom edge flush
      }

      bar.position.set(x + ox, adjustedY + oy, -HUD_DISTANCE);
      bar.rotation.set(0, 0, 0);
    }

    // Position elements
    for (const mesh of this.meshes) {
      const anchor: HudAnchor = mesh.userData.hudAnchor;
      const ox = mesh.userData.hudOffsetX ?? 0;
      const oy = mesh.userData.hudOffsetY ?? 0;
      const { x, y } = this.anchorPosition(anchor, halfW, halfH);

      mesh.position.set(x + ox, y + oy, -HUD_DISTANCE);
      mesh.rotation.set(0, 0, 0);
    }
  }

  private anchorPosition(anchor: HudAnchor, halfW: number, halfH: number, margin = MARGIN): { x: number; y: number } {
    switch (anchor) {
      case 'top-left':    return { x: -halfW + margin, y: halfH - margin };
      case 'top-center':  return { x: 0, y: halfH - margin };
      case 'top-right':   return { x: halfW - margin, y: halfH - margin };
      case 'bottom-left': return { x: -halfW + margin, y: -halfH + margin };
      case 'bottom-center': return { x: 0, y: -halfH + margin };
      case 'bottom-right': return { x: halfW - margin, y: -halfH + margin };
      case 'center':      return { x: 0, y: 0 };
    }
  }

  private clearMeshes(): void {
    for (const mesh of this.meshes) {
      this.hudGroup.remove(mesh);
      mesh.traverse((obj) => {
        if (obj instanceof THREE.Mesh) {
          obj.geometry?.dispose();
          if (Array.isArray(obj.material))
            obj.material.forEach((m: THREE.Material) => m.dispose());
          else obj.material?.dispose();
        }
      });
    }
    for (const bar of this.barMeshes) {
      this.hudGroup.remove(bar);
      bar.geometry?.dispose();
      if (Array.isArray(bar.material))
        bar.material.forEach((m) => m.dispose());
      else bar.material?.dispose();
    }
    this.meshes = [];
    this.barMeshes = [];
  }
}
