const PITCH_LIMIT = 1.4;
const FIRE_EVERY_MS = 280;
const PAD_LOOK_SPEED = 2.6;

/**
 * Looking and shooting for Paint Blasters: mouse with pointer lock, a
 * gamepad's right stick, or a thumb dragged across the screen. Movement still
 * comes from the shared PartyInput stick.
 */
export class ShooterControls {
  yaw = 0;
  pitch = 0;
  enabled = false;
  locked = false;
  private element: HTMLElement | null = null;
  private firing = false;
  private lastShot = 0;
  private jumpKey = false;
  private crouchKey = false;
  private padJump = false;
  private padCrouch = false;
  private padFire = false;

  constructor(
    private sensitivity = 0.0022,
    private onLockChange: (locked: boolean) => void = () => {},
  ) {
    document.addEventListener('pointerlockchange', this.onPointerLock);
    window.addEventListener('keydown', this.onKeyDown);
    window.addEventListener('keyup', this.onKeyUp);
    window.addEventListener('blur', this.onBlur);
  }

  attach(element: HTMLElement) {
    this.detach();
    this.element = element;
    element.addEventListener('mousedown', this.onMouseDown);
    element.addEventListener('mouseup', this.onMouseUp);
    element.addEventListener('contextmenu', this.onContextMenu);
    document.addEventListener('mousemove', this.onMouseMove);
  }

  /** Grab the pointer so the mouse turns the view instead of moving a cursor. */
  lock() {
    if (!this.enabled || this.locked) return;
    void this.element?.requestPointerLock?.();
  }

  release() {
    if (document.pointerLockElement === this.element) document.exitPointerLock();
  }

  /** A thumb dragged across the screen, in pixels since the last frame. */
  look(dx: number, dy: number) {
    if (!this.enabled) return;
    this.turn(dx * 2.2, dy * 2.2);
  }

  pullTrigger(down: boolean) {
    this.firing = down;
    if (down) this.lastShot = 0;
  }

  /** True at most once per shot, so holding the trigger fires at a fixed rate. */
  takeShot(now: number) {
    if (!this.enabled) return false;
    const wants = this.firing || this.padFire;
    if (!wants || now - this.lastShot < FIRE_EVERY_MS) return false;
    this.lastShot = now;
    return true;
  }

  get jump() {
    return this.jumpKey || this.padJump;
  }

  get crouch() {
    return this.crouchKey || this.padCrouch;
  }

  /** The on-screen crouch toggle for touch. */
  setCrouch(down: boolean) {
    this.crouchKey = down;
  }

  /** The right stick and the triggers, read once a frame. */
  readGamepad(dt: number) {
    const pad = typeof navigator.getGamepads === 'function' ? Array.from(navigator.getGamepads()).find(Boolean) : null;
    if (!pad) {
      this.padFire = false;
      this.padJump = false;
      this.padCrouch = false;
      return;
    }
    const [, , lookX = 0, lookY = 0] = pad.axes;
    if (Math.hypot(lookX, lookY) > 0.18) {
      this.yaw -= lookX * PAD_LOOK_SPEED * dt;
      this.pitch -= lookY * PAD_LOOK_SPEED * dt;
      this.clampPitch();
    }
    this.padFire = !!pad.buttons[7]?.pressed || !!pad.buttons[5]?.pressed;
    this.padJump = !!pad.buttons[0]?.pressed;
    this.padCrouch = !!pad.buttons[1]?.pressed || !!pad.buttons[10]?.pressed;
  }

  private turn(dx: number, dy: number) {
    this.yaw -= dx * this.sensitivity;
    this.pitch -= dy * this.sensitivity;
    this.clampPitch();
  }

  private clampPitch() {
    this.pitch = Math.min(PITCH_LIMIT, Math.max(-PITCH_LIMIT, this.pitch));
  }

  private onPointerLock = () => {
    this.locked = document.pointerLockElement === this.element;
    if (!this.locked) this.firing = false;
    this.onLockChange(this.locked);
  };

  private onMouseMove = (event: MouseEvent) => {
    if (!this.enabled || !this.locked) return;
    this.turn(event.movementX, event.movementY);
  };

  private onMouseDown = (event: MouseEvent) => {
    if (!this.enabled || event.button !== 0) return;
    if (!this.locked) {
      this.lock();
      return;
    }
    this.pullTrigger(true);
  };

  private onMouseUp = (event: MouseEvent) => {
    if (event.button === 0) this.pullTrigger(false);
  };

  private onContextMenu = (event: Event) => {
    if (this.enabled) event.preventDefault();
  };

  private onKeyDown = (event: KeyboardEvent) => {
    if (!this.enabled) return;
    if (event.code === 'Space') {
      this.jumpKey = true;
      event.preventDefault();
    }
    if (event.code === 'ControlLeft' || event.code === 'ControlRight' || event.code === 'KeyC') {
      this.crouchKey = true;
      event.preventDefault();
    }
  };

  private onKeyUp = (event: KeyboardEvent) => {
    if (event.code === 'Space') this.jumpKey = false;
    if (event.code === 'ControlLeft' || event.code === 'ControlRight' || event.code === 'KeyC') this.crouchKey = false;
  };

  private onBlur = () => {
    this.jumpKey = false;
    this.crouchKey = false;
    this.firing = false;
  };

  private detach() {
    if (!this.element) return;
    this.element.removeEventListener('mousedown', this.onMouseDown);
    this.element.removeEventListener('mouseup', this.onMouseUp);
    this.element.removeEventListener('contextmenu', this.onContextMenu);
    document.removeEventListener('mousemove', this.onMouseMove);
    this.element = null;
  }

  dispose() {
    this.release();
    this.detach();
    document.removeEventListener('pointerlockchange', this.onPointerLock);
    window.removeEventListener('keydown', this.onKeyDown);
    window.removeEventListener('keyup', this.onKeyUp);
    window.removeEventListener('blur', this.onBlur);
  }
}
