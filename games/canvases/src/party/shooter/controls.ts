import { bound, settings } from '../settings';
import {isGameplayTyping}from'../typing';

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
  private capturing = false;
  private captureView: string | null = null;
  private releasing = false;
  private cursorReleased = false;
  private requesting = false;
  private element: HTMLElement | null = null;
  private firing = false;
  private actionKey=false;
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
    window.addEventListener('focus', this.onFocus);
    document.addEventListener('focusin',this.onFocusIn);
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
    if (isGameplayTyping() || (!this.enabled && !this.capturing) || !this.element || this.requesting) return;
    if (document.pointerLockElement === this.element) return;
    this.cursorReleased = false;
    // Safari rejects this when the window is not focused, or when the element
    // went away between the click and the request.
    this.requesting = true;
    try {
      void Promise.resolve(this.element.requestPointerLock?.()).catch(() => {}).finally(() => {
        this.requesting = false;
      });
    } catch {
      this.requesting = false;
    }
  }

  /** Board items share cursor capture with bouts, but handle their own aim. */
  capture(active: boolean, view = 'first-person') {
    if (active === this.capturing && (!active || view === this.captureView)) return;
    this.capturing = active;
    this.captureView = active ? view : null;
    this.cursorReleased = false;
    if (active) this.lock();
    else this.release();
  }

  release() {
    if (document.pointerLockElement === this.element) {
      this.releasing = true;
      document.exitPointerLock();
    }
  }

  /** A thumb dragged across the screen, in pixels since the last frame. */
  look(dx: number, dy: number) {
    if (!this.enabled || isGameplayTyping()) return;
    this.turn(dx * 2.2, dy * 2.2);
  }

  pullTrigger(down: boolean) {
    this.firing = down && !isGameplayTyping();
    if (down) this.lastShot = 0;
  }

  /** True at most once per shot, so holding the trigger fires at a fixed rate. */
  takeShot(now: number) {
    if (isGameplayTyping()){this.clearHeld();return false;}
    if (!this.enabled) return false;
    const wants = this.firing || this.padFire;
    if (!wants || now - this.lastShot < FIRE_EVERY_MS) return false;
    this.lastShot = now;
    return true;
  }

  get holdingAction() {return !isGameplayTyping()&&(this.firing || this.padFire || this.actionKey);}

  get jump() {
    return !isGameplayTyping()&&(this.jumpKey || this.padJump);
  }

  get crouch() {
    return !isGameplayTyping()&&(this.crouchKey || this.padCrouch);
  }

  /** The on-screen crouch toggle for touch. */
  setCrouch(down: boolean) {
    this.crouchKey = down && !isGameplayTyping();
  }

  /** The right stick and the triggers, read once a frame. */
  readGamepad(dt: number) {
    if(isGameplayTyping()){this.clearHeld();return;}
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
    const wasLocked = this.locked;
    this.locked = document.pointerLockElement === this.element;
    const releasedByView = !this.locked && this.releasing;
    if (!this.locked) this.releasing = false;
    // A request can finish after the action ended or the canvas was removed.
    if (this.locked && ((!this.capturing && !this.enabled)||isGameplayTyping())) this.release();
    if (!this.locked) {
      this.firing = false;this.actionKey=false;
      if (wasLocked && !releasedByView && document.hasFocus()) this.cursorReleased = true;
      // Switching views can finish before the browser completes its previous
      // unlock. Acquire the new view after that handoff, without another click.
      if (releasedByView) setTimeout(() => {
        if (this.capturing && !this.cursorReleased) this.lock();
      }, 0);
    }
    this.onLockChange(this.locked);
  };

  private onFocus = () => {
    if (this.capturing && !this.cursorReleased) this.lock();
  };

  private onMouseMove = (event: MouseEvent) => {
    if (!this.enabled || !this.locked || isGameplayTyping(event.target)) return;
    const { sensitivity, invertY } = settings().camera;
    this.turn(event.movementX * sensitivity, event.movementY * sensitivity * (invertY ? -1 : 1));
  };

  private onMouseDown = (event: MouseEvent) => {
    if (!this.enabled || event.button !== 0 || isGameplayTyping(event.target)) return;
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
    if (this.enabled&&!isGameplayTyping(event.target)) event.preventDefault();
  };

  private onKeyDown = (event: KeyboardEvent) => {
    if(event.isComposing||isGameplayTyping(event.target)){this.clearHeld();return;}
    if (!this.enabled) return;
    if(bound('action',event.code))this.actionKey=true;
    if (bound('jump', event.code)) {
      this.jumpKey = true;
      event.preventDefault();
    }
    if (bound('crouch', event.code)) {
      this.crouchKey = true;
      event.preventDefault();
    }
  };

  private onKeyUp = (event: KeyboardEvent) => {
    if(bound('action',event.code))this.actionKey=false;
    if (bound('jump', event.code)) this.jumpKey = false;
    if (bound('crouch', event.code)) this.crouchKey = false;
  };

  /** Release every keyboard, pointer and cached controller action on UI focus. */
  clearHeld(){
    this.actionKey=false;this.jumpKey=false;this.crouchKey=false;this.firing=false;
    this.padJump=false;this.padCrouch=false;this.padFire=false;
  }
  private onBlur=()=>this.clearHeld();
  private onFocusIn=(event:FocusEvent)=>{
    if(isGameplayTyping(event.target)){this.clearHeld();this.release();}
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
    this.enabled = false;
    this.capturing = false;
    this.release();
    this.detach();
    document.removeEventListener('pointerlockchange', this.onPointerLock);
    window.removeEventListener('keydown', this.onKeyDown);
    window.removeEventListener('keyup', this.onKeyUp);
    window.removeEventListener('blur', this.onBlur);
    window.removeEventListener('focus', this.onFocus);
    document.removeEventListener('focusin',this.onFocusIn);
    this.clearHeld();
  }
}
