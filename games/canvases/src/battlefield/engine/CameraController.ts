import * as THREE from 'three';

export type CameraModeName =
  | 'deck_builder'
  | 'draft'
  | 'pack_summary'
  | 'pack_reveal'
  | 'pack_tear'
  | 'deck_select'
  | 'search'
  | 'scry'
  | 'orbit'
  | 'manual';

interface CameraState {
  position: THREE.Vector3;
  lookAt: THREE.Vector3;
  up: THREE.Vector3;
  fov: number;
  /** When true, position is set directly instead of lerped (e.g. orbit uses set()) */
  direct?: boolean;
  /** Custom lerp factor for FOV (default: snap) */
  fovLerp?: number;
}

export interface CameraUpdateContext {
  deckBuilderMode: boolean;
  cardBrowserMode?: boolean;
  isPackOpening: boolean;
  packPhase: string;
  packRevealZoomed: boolean;
  packTearProgress: number;
  deckSelectMode: boolean;
  searchActive: boolean;
  searchCameraTarget: THREE.Vector3 | null;
  searchCameraLook: THREE.Vector3 | null;
  scryCameraTarget: THREE.Vector3 | null;
  scryCameraLook: THREE.Vector3 | null;
  packCameraTarget?: THREE.Vector3 | null;
  packCameraLook?: THREE.Vector3 | null;
  isDraftMode: boolean;
  isLobby: boolean;
  players: { id: string }[];
  viewerId: string | null;
  moveState: {
    forward: boolean;
    back: boolean;
    left: boolean;
    right: boolean;
    up: boolean;
    down: boolean;
  };
  manualCamera: boolean;
}

export class CameraController {
  private camera: THREE.PerspectiveCamera;
  private activeMode: CameraModeName = 'orbit';
  private lastLoggedMode: CameraModeName | null = null;

  // Orbit state
  private currentAngle = 0;
  private radius = 18.0;
  private height = 6.8;
  private rotationStep = 0;
  private forceClockwise = false;
  // Last player count observed — used to reset radius/height when the
  // table size changes (e.g. joining/leaving players).
  private lastPlayerCount = 0;
  // Timestamp (ms) of the last manual zoom — the seat-angle auto-recenter
  // and the baseRadius/baseHeight lerp both wait this long before engaging.
  private lastManualZoomAt = 0;
  private static readonly MANUAL_ZOOM_HOLD_MS = 5000;

  // Timestamp (ms) of the last manual orbit drag — used to hold the
  // camera in its dragged position before snapping back to seat.
  private lastManualOrbitAt = 0;
  private static readonly MANUAL_ORBIT_HOLD_MS = 5000;

  // Tracks where the camera is looking (after lerp), used by inspect
  private currentLookAt = new THREE.Vector3();

  constructor(camera: THREE.PerspectiveCamera) {
    this.camera = camera;
  }

  public getActiveMode(): CameraModeName {
    return this.activeMode;
  }

  public getOrbitState() {
    return {
      angle: this.currentAngle,
      radius: this.radius,
      height: this.height,
    };
  }

  /**
   * Rotate the orbit by a horizontal pixel delta (from a right-click drag).
   * Marks the camera as manually positioned, so the seat auto-snap waits
   * MANUAL_ORBIT_HOLD_MS before resuming.
   */
  public orbitByDelta(dxPixels: number): void {
    const sensitivity = 0.005;
    this.currentAngle -= dxPixels * sensitivity;
    this.lastManualOrbitAt = performance.now();
    this.forceClockwise = false;
  }

  /**
   * Zoom the orbit by a wheel delta. Negative deltaY = wheel up = zoom in
   * (smaller radius), matching W/S behavior.
   */
  public zoomByDelta(deltaY: number): void {
    const sensitivity = 0.01;
    this.radius = THREE.MathUtils.clamp(
      this.radius + deltaY * sensitivity,
      10.0,
      30.0,
    );
    // Mark both zoom and orbit hold timestamps so the angle, radius, and
    // height all stay put for 5s and then reset back to the seat together.
    const now = performance.now();
    this.lastManualZoomAt = now;
    this.lastManualOrbitAt = now;
  }

  private isHoldingManualOrbit(): boolean {
    if (this.lastManualOrbitAt === 0) return false;
    return (performance.now() - this.lastManualOrbitAt) < CameraController.MANUAL_ORBIT_HOLD_MS;
  }

  /** Called when entering manual camera to capture current orbit params from camera position. */
  public enterManualCamera(): void {
    const pos = this.camera.position;
    this.radius = Math.sqrt(pos.x * pos.x + pos.z * pos.z);
    this.height = pos.y;
    this.currentAngle = Math.atan2(pos.z, pos.x);
  }

  /** Set the rotation step (e.g. from viewer index on join or V key). */
  public setRotationStep(step: number): void {
    this.rotationStep = step;
  }

  public getRotationStep(): number {
    return this.rotationStep;
  }

  /** Decrement rotation step and enable clockwise lerp (V key). */
  public rotateClockwise(playerCount: number): void {
    if (playerCount > 0) {
      this.rotationStep = (this.rotationStep - 1 + playerCount) % playerCount;
      this.forceClockwise = true;
    }
  }

  /** Force-set the angle (e.g. on first join). */
  public setAngle(angle: number): void {
    this.currentAngle = angle;
  }

  // ── Mode compute methods ──────────────────────────────────────────────

  private computeDraft(): CameraState {
    return {
      // Draft mode needs extra headroom because the drafted-card wings extend
      // farther toward screen-top than the pyramid itself.
      position: new THREE.Vector3(0, 7.8, 0.01),
      lookAt: new THREE.Vector3(-1.1, 0, 0),
      up: new THREE.Vector3(-1, 0, 0),
      fov: 72,
    };
  }

  private computeDeckBuilder(isBrowser = false): CameraState {
    // Browser mode: shift the lookAt toward world -X so the ring and honeycomb
    // (which sit around origin/negative-X in world space) render lower on screen,
    // leaving breathing room under the top HUD bar. `up = (-1,0,0)` means world
    // -X is screen up, so shifting lookAt toward -X visually pushes content down.
    const lookAtX = isBrowser ? -2.3 : 0;
    return {
      position: new THREE.Vector3(0, 8, 0.01),
      lookAt: new THREE.Vector3(lookAtX, 0, 0),
      up: new THREE.Vector3(-1, 0, 0),
      fov: 60,
    };
  }

  private computePackSummary(): CameraState {
    return {
      position: new THREE.Vector3(0, 8, 0.01),
      lookAt: new THREE.Vector3(0, 0, 0),
      up: new THREE.Vector3(-1, 0, 0),
      fov: 50,
    };
  }

  private computePackReveal(
    seatAngle: number,
    zoomed: boolean,
    tableScale: number,
  ): CameraState {
    const zoomIn = zoomed ? 1 : 0;
    const zoomEase = 1 - Math.pow(1 - zoomIn, 2.2);
    const packRadius = THREE.MathUtils.lerp(9.5, 7.8, zoomEase) * tableScale;
    return {
      position: new THREE.Vector3(
        Math.cos(seatAngle) * packRadius,
        THREE.MathUtils.lerp(5.8, 4.8, zoomEase),
        Math.sin(seatAngle) * packRadius,
      ),
      lookAt: new THREE.Vector3(0, 1.5, 0),
      up: new THREE.Vector3(0, 1, 0),
      fov: THREE.MathUtils.lerp(50, 44, zoomEase),
      fovLerp: 0.06,
    };
  }

  private computePackTear(
    seatAngle: number,
    packPhase: string,
    tearProgress: number,
    tableScale: number,
  ): CameraState {
    const zoomProgress =
      packPhase === 'burst'
        ? 1
        : packPhase === 'tear' || packPhase === 'grab'
          ? THREE.MathUtils.clamp(tearProgress, 0, 1)
          : 0;
    const zoomEase = 1 - Math.pow(1 - zoomProgress, 2.2);
    const packRadius = THREE.MathUtils.lerp(9.5, 7.1, zoomEase) * tableScale;
    return {
      position: new THREE.Vector3(
        Math.cos(seatAngle) * packRadius,
        THREE.MathUtils.lerp(5.8, 7.0, zoomEase),
        Math.sin(seatAngle) * packRadius,
      ),
      lookAt: new THREE.Vector3(
        0,
        THREE.MathUtils.lerp(0.5, 0.72, zoomEase),
        0,
      ),
      up: new THREE.Vector3(0, 1, 0),
      fov: THREE.MathUtils.lerp(50, 42, zoomEase),
    };
  }

  private computeDeckSelect(): CameraState {
    return {
      position: new THREE.Vector3(0, 8, 0.01),
      // Small downward shift so the ring sits just under the top HUD bar.
      lookAt: new THREE.Vector3(-0.8, 0, 0),
      up: new THREE.Vector3(-1, 0, 0),
      fov: 60,
    };
  }

  private computeSearch(
    target: THREE.Vector3,
    look: THREE.Vector3,
  ): CameraState {
    return {
      position: target,
      lookAt: look,
      up: new THREE.Vector3(-1, 0, 0),
      fov: 50,
    };
  }

  private computeScry(target: THREE.Vector3, look: THREE.Vector3): CameraState {
    return {
      position: target,
      lookAt: look,
      up: new THREE.Vector3(0, 1, 0),
      fov: 60,
    };
  }

  private computeOrbit(delta: number, ctx: CameraUpdateContext): CameraState {
    const turnSpeed = 1.6;
    const zoomSpeed = 6.0;

    // Target radius/height derived from player count. Pulled in closer
    // so 2p doesn't look lost in space.
    const pc = Math.max(2, ctx.players.length || 2);
    const tableScale = Math.min(1.6, Math.max(0.7, 0.35 * pc));
    const baseRadius = 9.0 + tableScale * 5.5; // ~12.85 @ 2p, ~14.78 @ 3p, ~17.8 @ 6p
    const baseHeight = 4.2 + tableScale * 1.0; // ~4.9 @ 2p, ~5.25 @ 3p, ~5.8 @ 6p

    // If player count just changed, mark the change — the radius/height
    // will lerp to the new base via the normal orbit path (respecting the
    // 5s manual-zoom hold).
    if (this.lastPlayerCount !== pc) {
      this.lastPlayerCount = pc;
    }

    const minRadius = Math.max(8.0, baseRadius - 6.0);
    const maxRadius = Math.min(30.0, baseRadius + 6.0);
    const minHeight = 4.6;
    const maxHeight = Math.max(7.4, baseHeight + 0.5);

    if (ctx.manualCamera) {
      if (ctx.moveState.left) this.currentAngle += turnSpeed * delta;
      if (ctx.moveState.right) this.currentAngle -= turnSpeed * delta;
      if (ctx.moveState.forward) this.radius -= zoomSpeed * delta;
      if (ctx.moveState.back) this.radius += zoomSpeed * delta;
      if (ctx.moveState.up) this.height += zoomSpeed * delta;
      if (ctx.moveState.down) this.height -= zoomSpeed * delta;
      this.radius = THREE.MathUtils.clamp(this.radius, minRadius, maxRadius);
      this.height = THREE.MathUtils.clamp(this.height, minHeight, maxHeight);
    } else {
      if (!ctx.isLobby && ctx.moveState.left)
        this.currentAngle += turnSpeed * delta;
      if (!ctx.isLobby && ctx.moveState.right)
        this.currentAngle -= turnSpeed * delta;
      if (ctx.moveState.forward) {
        this.radius -= zoomSpeed * delta;
        const now = performance.now();
        this.lastManualZoomAt = now;
        this.lastManualOrbitAt = now;
      }
      if (ctx.moveState.back) {
        this.radius += zoomSpeed * delta;
        const now = performance.now();
        this.lastManualZoomAt = now;
        this.lastManualOrbitAt = now;
      }
      // When not zooming AND the manual zoom hold window has elapsed, ease
      // radius/height toward the base derived from table size so changes in
      // player count reposition the camera.
      const heldZoom =
        performance.now() - this.lastManualZoomAt <
        CameraController.MANUAL_ZOOM_HOLD_MS;
      if (!ctx.moveState.forward && !ctx.moveState.back && !heldZoom) {
        this.radius += (baseRadius - this.radius) * Math.min(1, delta * 1.2);
        this.height += (baseHeight - this.height) * Math.min(1, delta * 1.2);
      }
      this.radius = THREE.MathUtils.clamp(this.radius, minRadius, maxRadius);
      this.height = THREE.MathUtils.clamp(this.height, 5.2, maxHeight);

      const seatStep =
        ctx.players.length > 0
          ? (Math.PI * 2) / ctx.players.length
          : Math.PI * 2;

      if (!ctx.moveState.left && !ctx.moveState.right && !this.isHoldingManualOrbit()) {
        const baseTargetAngle =
          ctx.players.length > 0 ? this.rotationStep * seatStep : 0;
        if (this.forceClockwise) {
          let targetAngle = baseTargetAngle;
          while (targetAngle >= this.currentAngle) {
            targetAngle -= Math.PI * 2;
          }
          const diff = targetAngle - this.currentAngle;
          this.currentAngle += diff * 0.18;
          if (Math.abs(diff) < 0.01) this.forceClockwise = false;
        } else {
          let diff = baseTargetAngle - this.currentAngle;
          diff = ((diff + Math.PI) % (Math.PI * 2)) - Math.PI;
          this.currentAngle += diff * 0.12;
        }
      } else if (ctx.players.length > 0) {
        // Manual orbit/zoom in progress — don't reassign rotationStep to
        // whatever seat the camera is currently pointing at. The viewer's
        // seat is set once (on join) and should only change via explicit
        // actions (V key / rotateClockwise). After the hold ends, the
        // camera will lerp back to the viewer's original seat.
        this.forceClockwise = false;
      }
    }

    return {
      position: new THREE.Vector3(
        Math.cos(this.currentAngle) * this.radius,
        this.height,
        Math.sin(this.currentAngle) * this.radius,
      ),
      lookAt: new THREE.Vector3(0, 0, 0),
      up: new THREE.Vector3(0, 1, 0),
      fov: 45,
      direct: true, // orbit uses position.set, not lerp
    };
  }

  // ── Main update ───────────────────────────────────────────────────────

  public update(delta: number, ctx: CameraUpdateContext): void {
    let state: CameraState;

    if (ctx.isDraftMode && !ctx.manualCamera) {
      this.activeMode = 'draft';
      state = this.computeDraft();
    } else if (ctx.deckBuilderMode && !ctx.manualCamera) {
      this.activeMode = 'deck_builder';
      state = this.computeDeckBuilder(ctx.cardBrowserMode === true);
    } else if (ctx.isPackOpening && !ctx.manualCamera) {
      const viewerIndex = ctx.players.findIndex((p) => p.id === ctx.viewerId);
      const seatAngle =
        viewerIndex >= 0 && ctx.players.length > 0
          ? (viewerIndex / ctx.players.length) * Math.PI * 2
          : 0;
      const packPlayerCount = Math.max(2, ctx.players.length || 2);
      const packTableScale = Math.min(
        1.6,
        Math.max(0.7, 0.35 * packPlayerCount),
      );
      // Clear any held manual-orbit state so the orbit camera can snap
      // back cleanly when the player exits pack view.
      this.lastManualOrbitAt = 0;
      this.lastManualZoomAt = 0;
      this.currentAngle = seatAngle;

      if (ctx.packPhase === 'summary') {
        this.activeMode = 'pack_summary';
        state = this.computePackSummary();
      } else if (ctx.packPhase === 'reveal') {
        this.activeMode = 'pack_reveal';
        state = this.computePackReveal(
          seatAngle,
          ctx.packRevealZoomed,
          packTableScale,
        );
      } else {
        this.activeMode = 'pack_tear';
        state = this.computePackTear(
          seatAngle,
          ctx.packPhase,
          ctx.packTearProgress,
          packTableScale,
        );
      }
    } else if (ctx.deckSelectMode && !ctx.manualCamera) {
      this.activeMode = 'deck_select';
      state = this.computeDeckSelect();
    } else if (
      ctx.searchActive &&
      ctx.searchCameraTarget &&
      ctx.searchCameraLook &&
      !ctx.manualCamera
    ) {
      this.activeMode = 'search';
      state = this.computeSearch(ctx.searchCameraTarget, ctx.searchCameraLook);
    } else if (
      !ctx.isLobby &&
      ctx.scryCameraTarget &&
      ctx.scryCameraLook &&
      !ctx.manualCamera
    ) {
      this.activeMode = 'scry';
      state = this.computeScry(ctx.scryCameraTarget, ctx.scryCameraLook);
    } else {
      this.activeMode = ctx.manualCamera ? 'manual' : 'orbit';
      state = this.computeOrbit(delta, ctx);
    }

    if (this.activeMode !== this.lastLoggedMode) {
      // TEMP: diagnosing ring-size flicker on turn transitions.
      console.log(
        '[camera mode]',
        this.lastLoggedMode,
        '->',
        this.activeMode,
        'fov:',
        state.fov,
      );
      this.lastLoggedMode = this.activeMode;
    }

    // Apply state to camera
    this.camera.up.lerp(state.up, 0.12).normalize();

    if (state.fovLerp !== undefined) {
      // Smooth FOV transition (pack reveal)
      this.camera.fov = THREE.MathUtils.lerp(
        this.camera.fov,
        state.fov,
        state.fovLerp,
      );
      this.camera.updateProjectionMatrix();
    } else if (Math.abs(this.camera.fov - state.fov) > 0.01) {
      this.camera.fov = state.fov;
      this.camera.updateProjectionMatrix();
    }

    if (state.direct) {
      this.camera.position.copy(state.position);
    } else {
      this.camera.position.lerp(state.position, 0.12);
    }

    this.camera.lookAt(state.lookAt);
    this.currentLookAt.copy(state.lookAt);
  }

  // ── Inspect placement ─────────────────────────────────────────────────

  public getInspectPlacement(): {
    position: THREE.Vector3;
    rotation: THREE.Euler;
    scale: number;
  } {
    const mode = this.activeMode;
    if (
      mode === 'deck_builder' ||
      mode === 'draft' ||
      mode === 'pack_summary' ||
      mode === 'deck_select' ||
      mode === 'search'
    ) {
      return this.computeTopDownInspect();
    }
    return this.computePerspectiveInspect();
  }

  private computeTopDownInspect(): {
    position: THREE.Vector3;
    rotation: THREE.Euler;
    scale: number;
  } {
    const cam = this.camera;
    const camToLook = this.currentLookAt.clone().sub(cam.position);
    const distance = camToLook.length();

    // Position at 40% of the way from camera to lookAt
    const fraction = 0.4;
    const cardPos = cam.position
      .clone()
      .add(camToLook.clone().multiplyScalar(fraction));

    // Face the card toward the camera (+Z toward camera)
    const lookMat = new THREE.Matrix4().lookAt(cam.position, cardPos, cam.up);
    const q = new THREE.Quaternion().setFromRotationMatrix(lookMat);
    const rot = new THREE.Euler().setFromQuaternion(q, 'XYZ');

    const scale = Math.min(distance * fraction * 0.55, 3.0);

    return { position: cardPos, rotation: rot, scale };
  }

  private computePerspectiveInspect(): {
    position: THREE.Vector3;
    rotation: THREE.Euler;
    scale: number;
  } {
    const cam = this.camera;
    const camToLook = this.currentLookAt.clone().sub(cam.position);
    const distance = camToLook.length();

    // Place at 35% of camera-to-lookAt (same logic as top-down at 40%)
    const fraction = 0.35;
    const cardPos = cam.position
      .clone()
      .add(camToLook.clone().multiplyScalar(fraction));

    // Face card toward camera using lookAt (same clean approach as top-down inspect,
    // avoids the manual yaw+tilt calculation that caused the "tilted" appearance)
    const lookMat = new THREE.Matrix4().lookAt(
      cam.position,
      cardPos,
      new THREE.Vector3(0, 1, 0),
    );
    const q = new THREE.Quaternion().setFromRotationMatrix(lookMat);
    const rot = new THREE.Euler().setFromQuaternion(q, 'XYZ');

    const fovScale = 45 / cam.fov;
    const scale = Math.min(distance * fraction * 0.55 * fovScale, 4.0);

    return { position: cardPos, rotation: rot, scale };
  }
}
