import type { Button, Input } from './contract';

/** An input with every field filled in, as this sends it. */
export type FullInput = Required<Omit<Input, 'aim'>> & { aim: Input['aim'] } & { held: Record<Button, boolean>; presses: Record<Button, number> };
import { gamepad, settings } from './party/settings';
import { isGameplayTyping } from './party/typing';

type Direction = [number, number];

function direction(code: string): Direction | null {
  const { keys } = settings();
  if (keys.up.includes(code)) return [0, -1];
  if (keys.down.includes(code)) return [0, 1];
  if (keys.left.includes(code)) return [-1, 0];
  if (keys.right.includes(code)) return [1, 0];
  return null;
}

const ALT_KEYS = ['ShiftLeft', 'ShiftRight', 'KeyE'];
const ALT_PAD = 1;

function buttonFor(code: string): Button | null {
  if (settings().keys.action.includes(code)) return 'action';
  if (ALT_KEYS.includes(code)) return 'alt';
  return null;
}

export type AimSource = {
  /** The world point under the pointer, or null off the canvas. */
  ground(clientX: number, clientY: number): { x: number; z: number } | null;
  /** Where the viewer's character is now. */
  me(): { x: number; z: number } | null;
};

type Held = Record<Button, { key: boolean; mouse: boolean; pad: boolean }>;
const released = (): Held => ({ action: { key: false, mouse: false, pad: false }, alt: { key: false, mouse: false, pad: false } });

/**
 * Keyboard and mouse first, a controller second. WASD or the arrows move,
 * Space or a left click acts, Shift or E is the second button (a right click
 * too, though nothing needs it), and the mouse aims at the floor point under
 * it. Screen up is away from the camera (-z).
 */
export class MadeControls {
  enabled = false;
  private keys = new Set<string>();
  private held = released();
  private presses: Record<Button, number> = { action: 0, alt: 0 };
  private aim: { x: number; z: number } | null = null;
  private seq = 0;
  private pointer: { x: number; y: number } | null = null;
  private padDown: Record<Button, boolean> = { action: false, alt: false };

  private press(button: Button) {
    this.presses[button] += 1;
  }

  private onKeyDown = (event: KeyboardEvent) => {
    if (event.isComposing || isGameplayTyping(event.target)) {
      this.clear();
      return;
    }
    if (!this.enabled) return;
    if (direction(event.code)) {
      this.keys.add(event.code);
      event.preventDefault();
    }
    const button = buttonFor(event.code);
    if (button) {
      if (!event.repeat && !this.held[button].key) this.press(button);
      this.held[button].key = true;
      event.preventDefault();
    }
  };
  private onKeyUp = (event: KeyboardEvent) => {
    this.keys.delete(event.code);
    const button = buttonFor(event.code);
    if (button) this.held[button].key = false;
  };
  private onPointerDown = (event: PointerEvent) => {
    this.pointer = { x: event.clientX, y: event.clientY };
    const button: Button | null = event.button === 0 ? 'action' : event.button === 2 ? 'alt' : null;
    if (!this.enabled || !button) return;
    this.press(button);
    this.held[button].mouse = true;
  };
  private onPointerUp = (event: PointerEvent) => {
    if (event.button === 0) this.held.action.mouse = false;
    if (event.button === 2) this.held.alt.mouse = false;
  };
  private onContextMenu = (event: MouseEvent) => {
    if (this.enabled) event.preventDefault();
  };
  private onPointerMove = (event: PointerEvent) => {
    this.pointer = { x: event.clientX, y: event.clientY };
  };
  private onPointerLeave = () => {
    this.pointer = null;
  };
  private onBlur = () => this.clear();

  constructor(
    private readonly surface: HTMLElement,
    private readonly aimSource: AimSource,
  ) {
    window.addEventListener('keydown', this.onKeyDown);
    window.addEventListener('keyup', this.onKeyUp);
    window.addEventListener('blur', this.onBlur);
    window.addEventListener('pointerup', this.onPointerUp);
    surface.addEventListener('pointerdown', this.onPointerDown);
    surface.addEventListener('pointermove', this.onPointerMove);
    surface.addEventListener('pointerleave', this.onPointerLeave);
    surface.addEventListener('contextmenu', this.onContextMenu);
  }

  private clear() {
    this.keys.clear();
    this.held = released();
  }

  /** The controls to send now. */
  read(): FullInput {
    const presses = { ...this.presses };
    if (!this.enabled || isGameplayTyping()) {
      this.clear();
      return { seq: this.seq, move: { x: 0, z: 0 }, aim: this.aim, held: { action: false, alt: false }, presses };
    }
    let x = 0;
    let z = 0;
    for (const code of this.keys) {
      const [dx, dz] = direction(code) ?? [0, 0];
      x += dx;
      z += dz;
    }
    const pad = gamepad();
    if (pad) {
      const [px = 0, pz = 0, rx = 0, rz = 0] = pad.axes;
      if (Math.hypot(px, pz) > settings().pad.deadzone) {
        x += px;
        z += pz;
      }
      const me = this.aimSource.me();
      if (me && Math.hypot(rx, rz) > 0.35) {
        const toward = unit(rx, rz);
        this.aim = { x: me.x + toward.x * PAD_AIM_REACH, z: me.z + toward.z * PAD_AIM_REACH };
      }
      for (const [button, index] of [['action', settings().pad.action], ['alt', ALT_PAD]] as [Button, number][]) {
        const down = !!pad.buttons[index]?.pressed;
        if (down && !this.padDown[button]) this.press(button);
        this.padDown[button] = down;
        this.held[button].pad = down;
      }
    }
    if (this.pointer) {
      const point = this.aimSource.ground(this.pointer.x, this.pointer.y);
      if (point) this.aim = point;
    }
    const length = Math.hypot(x, z);
    const move = length > 1 ? { x: x / length, z: z / length } : { x, z };
    const isHeld = (button: Button) => Object.values(this.held[button]).some(Boolean);
    return { seq: this.seq, move, aim: this.aim, held: { action: isHeld('action'), alt: isHeld('alt') }, presses };
  }

  /** Numbers each input that goes out, so the relay can tell old from new. */
  sent() {
    this.seq += 1;
  }

  dispose() {
    window.removeEventListener('keydown', this.onKeyDown);
    window.removeEventListener('keyup', this.onKeyUp);
    window.removeEventListener('blur', this.onBlur);
    window.removeEventListener('pointerup', this.onPointerUp);
    this.surface.removeEventListener('pointerdown', this.onPointerDown);
    this.surface.removeEventListener('pointermove', this.onPointerMove);
    this.surface.removeEventListener('pointerleave', this.onPointerLeave);
    this.surface.removeEventListener('contextmenu', this.onContextMenu);
    this.clear();
  }
}

function unit(x: number, z: number) {
  const length = Math.hypot(x, z) || 1;
  return { x: x / length, z: z / length };
}

/** A controller's right stick aims this far ahead of the character. */
const PAD_AIM_REACH = 4;

/** Whether sending `next` after `previous` would change nothing the game reads. */
export function sameInput(previous: FullInput | null, next: FullInput) {
  if (!previous) return false;
  const close = (a: number, b: number, by: number) => Math.abs(a - b) < by;
  const aimSame =
    previous.aim === next.aim ||
    (!!previous.aim && !!next.aim && close(previous.aim.x, next.aim.x, 0.01) && close(previous.aim.z, next.aim.z, 0.01));
  return (
    close(previous.move.x, next.move.x, 0.01) &&
    close(previous.move.z, next.move.z, 0.01) &&
    aimSame &&
    previous.held.action === next.held.action &&
    previous.held.alt === next.held.alt &&
    previous.presses.action === next.presses.action &&
    previous.presses.alt === next.presses.alt
  );
}
