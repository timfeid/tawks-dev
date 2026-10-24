import * as THREE from 'three';

export type PointerState = {
  x: number;
  y: number; // Normalized -1 to 1
  rawX: number;
  rawY: number;
  isDown: boolean;
  movementX: number;
  movementY: number;
};

export class InputManager {
  public pointer: PointerState = { x: 0, y: 0, rawX: 0, rawY: 0, isDown: false, movementX: 0, movementY: 0 };
  public raycaster = new THREE.Raycaster();
  
  private canvas: HTMLCanvasElement;
  private camera: THREE.Camera;
  private isDragging = false;
  private downPos = { x: 0, y: 0 };
  private downButton = 0;
  private dragThreshold = 5;

  // Events
  public onClick?: (intersect: THREE.Intersection | null, button: number) => void;
  public onDragStart?: (intersect: THREE.Intersection | null) => void;
  public onDragMove?: (intersect: THREE.Intersection | null) => void;
  public onDragEnd?: () => void;
  public onMiddleDrag?: (dx: number, dy: number) => void;
  public onWheel?: (deltaY: number) => void;
  public getInteractables?: () => THREE.Object3D[];

  private isMiddleDragging = false;

  constructor(canvas: HTMLCanvasElement, camera: THREE.Camera) {
    this.canvas = canvas;
    this.camera = camera;
    this.raycaster.layers.enableAll();
    
    // Bind events
    this.canvas.addEventListener('pointermove', this.onPointerMove);
    this.canvas.addEventListener('pointerdown', this.onPointerDown);
    this.canvas.addEventListener('pointerup', this.onPointerUp);
    this.canvas.addEventListener('pointercancel', this.onPointerCancel);
    this.canvas.addEventListener('contextmenu', (e) => e.preventDefault());
    this.canvas.addEventListener('wheel', this.handleWheel, { passive: false });
  }

  private handleWheel = (event: WheelEvent) => {
    if (!this.onWheel) return;
    event.preventDefault();
    this.onWheel(event.deltaY);
  };

  private updatePointer(event: PointerEvent) {
    const rect = this.canvas.getBoundingClientRect();
    this.pointer.rawX = event.clientX;
    this.pointer.rawY = event.clientY;
    this.pointer.x = ((event.clientX - rect.left) / rect.width) * 2 - 1;
    this.pointer.y = -((event.clientY - rect.top) / rect.height) * 2 + 1;
    this.pointer.movementX = event.movementX;
    this.pointer.movementY = event.movementY;
  }

  private onPointerMove = (event: PointerEvent) => {
    this.updatePointer(event);

    if (this.pointer.isDown && !this.isDragging && this.downButton === 0) {
      const dx = event.clientX - this.downPos.x;
      const dy = event.clientY - this.downPos.y;
      if (Math.sqrt(dx * dx + dy * dy) > this.dragThreshold) {
        this.isDragging = true;
        // Find what we are dragging
        this.raycastAndNotify('dragStart');
      }
    }

    // Middle-click drag → camera orbit
    if (this.pointer.isDown && this.downButton === 1) {
      const dx = event.clientX - this.downPos.x;
      const dy = event.clientY - this.downPos.y;
      if (!this.isMiddleDragging && Math.sqrt(dx * dx + dy * dy) > this.dragThreshold) {
        this.isMiddleDragging = true;
      }
      if (this.isMiddleDragging) {
        this.onMiddleDrag?.(event.movementX, event.movementY);
      }
    }

    if (this.isDragging) {
      // For drag move, we often want the point on a plane, not just object intersections
      // But we'll leave the plane logic to the controller via getDragPlane() if needed
      // For now, pass null or intersection
      this.raycastAndNotify('dragMove');
    }
  };

  private onPointerDown = (event: PointerEvent) => {
    this.updatePointer(event);
    this.pointer.isDown = true;
    this.downPos = { x: event.clientX, y: event.clientY };
    this.downButton = event.button;
    this.isDragging = false;

    // Prevent the browser's middle-click auto-scroll ("panner") cursor.
    if (event.button === 1) {
      event.preventDefault();
    }

    // Only capture pointer for left-click; right/middle clicks don't need capture
    // and setPointerCapture on button=2 causes pointercancel on some browsers.
    if (event.button === 0 && this.canvas.setPointerCapture) {
      this.canvas.setPointerCapture(event.pointerId);
    }
  };

  private onPointerUp = (event: PointerEvent) => {
    this.updatePointer(event);
    if (this.isDragging) {
      this.onDragEnd?.();
    } else if (!this.isMiddleDragging) {
      // Don't fire a click when we were middle-dragging the camera
      this.raycastAndNotify('click', event.button);
    }

    this.pointer.isDown = false;
    this.isDragging = false;
    this.isMiddleDragging = false;

    if (this.canvas.releasePointerCapture) {
      this.canvas.releasePointerCapture(event.pointerId);
    }
  };

  private onPointerCancel = (event: PointerEvent) => {
    this.pointer.isDown = false;
    if (this.isDragging) {
      this.onDragEnd?.();
    }
    this.isDragging = false;
  };

  public getIntersection(objects: THREE.Object3D[]) {
    this.raycaster.setFromCamera(new THREE.Vector2(this.pointer.x, this.pointer.y), this.camera);
    // Recursive by default for groups
    const hits = this.raycaster.intersectObjects(objects, true);
    return hits.length > 0 ? hits[0] : null;
  }

  private raycastAndNotify(type: 'click' | 'dragStart' | 'dragMove', button?: number) {
    const interactables = this.getInteractables?.() ?? [];
    const hit = interactables.length > 0 ? this.getIntersection(interactables) : null;
    if (type === 'click') {
      if (this.onClick) this.onClick(hit, button || 0);
    } else if (type === 'dragStart') {
      this.onDragStart?.(hit);
    } else if (type === 'dragMove') {
      this.onDragMove?.(hit);
    }
  }

  public dispose() {
    this.canvas.removeEventListener('pointermove', this.onPointerMove);
    this.canvas.removeEventListener('pointerdown', this.onPointerDown);
    this.canvas.removeEventListener('pointerup', this.onPointerUp);
    this.canvas.removeEventListener('pointercancel', this.onPointerCancel);
    this.canvas.removeEventListener('wheel', this.handleWheel);
  }
}
