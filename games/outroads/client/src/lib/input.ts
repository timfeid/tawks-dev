/** Keyboard, gamepad and touch merged into one set of car controls and one-shot actions. */

import type { Controls } from './physics';

export type Press = 'honk' | 'lights' | 'respawn' | 'camera' | 'photo' | 'map' | 'units';

export type TouchState = { left: boolean; right: boolean; gas: boolean; brake: boolean; drift: boolean };

const KEYS: Record<string, Press> = {
  KeyH: 'honk',
  KeyL: 'lights',
  KeyR: 'respawn',
  KeyC: 'camera',
  KeyP: 'photo',
  KeyM: 'map',
  KeyU: 'units',
};

const PAD_BUTTONS: [number, Press][] = [
  [1, 'honk'],
  [2, 'lights'],
  [3, 'respawn'],
  [5, 'camera'],
  [9, 'photo'],
];

export class Input {
  touch: TouchState = { left: false, right: false, gas: false, brake: false, drift: false };
  /** Fills when a key or pad button goes down; drained each frame. */
  presses: Press[] = [];
  enabled = true;

  private down = new Set<string>();
  private padWas = new Set<number>();
  private steerKeys = 0;

  constructor(private target: Window) {
    target.addEventListener('keydown', this.keydown);
    target.addEventListener('keyup', this.keyup);
    target.addEventListener('blur', this.blur);
  }

  private typing(event: KeyboardEvent) {
    const el = event.target as HTMLElement | null;
    return !!el && (el.tagName === 'INPUT' || el.tagName === 'TEXTAREA' || el.isContentEditable);
  }

  private keydown = (event: KeyboardEvent) => {
    if (this.typing(event) || !this.enabled) return;
    if (['ArrowUp', 'ArrowDown', 'ArrowLeft', 'ArrowRight', 'Space'].includes(event.code)) event.preventDefault();
    if (!event.repeat && KEYS[event.code]) this.presses.push(KEYS[event.code]);
    this.down.add(event.code);
  };

  private keyup = (event: KeyboardEvent) => {
    this.down.delete(event.code);
  };

  private blur = () => this.down.clear();

  private has(...codes: string[]) {
    return codes.some((c) => this.down.has(c));
  }

  /** Reads everything for this frame. Keyboard steering eases in so taps stay small. */
  read(dt: number): Controls {
    let throttle = this.has('KeyW', 'ArrowUp') || this.touch.gas ? 1 : 0;
    let brake = this.has('KeyS', 'ArrowDown') || this.touch.brake ? 1 : 0;
    let handbrake = this.has('Space') || this.touch.drift;
    const keySteer = (this.has('KeyD', 'ArrowRight') || this.touch.right ? 1 : 0) - (this.has('KeyA', 'ArrowLeft') || this.touch.left ? 1 : 0);
    this.steerKeys = keySteer === 0 ? 0 : Math.max(-1, Math.min(1, this.steerKeys + keySteer * dt * 6));
    if (keySteer !== 0 && Math.sign(this.steerKeys) !== keySteer) this.steerKeys = keySteer * 0.3;
    let steer = this.steerKeys;

    const pads = typeof navigator.getGamepads === 'function' ? navigator.getGamepads() : [];
    for (const pad of pads) {
      if (!pad) continue;
      const stick = pad.axes[0] ?? 0;
      if (Math.abs(stick) > 0.12) steer = Math.sign(stick) * ((Math.abs(stick) - 0.12) / 0.88) ** 1.4;
      throttle = Math.max(throttle, pad.buttons[7]?.value ?? 0, (pad.axes[1] ?? 0) < -0.5 ? 1 : 0);
      brake = Math.max(brake, pad.buttons[6]?.value ?? 0);
      handbrake ||= !!pad.buttons[0]?.pressed;
      if (this.enabled) {
        for (const [button, press] of PAD_BUTTONS) {
          const on = !!pad.buttons[button]?.pressed;
          const id = pad.index * 100 + button;
          if (on && !this.padWas.has(id)) this.presses.push(press);
          if (on) this.padWas.add(id);
          else this.padWas.delete(id);
        }
      }
    }
    if (!this.enabled) return { throttle: 0, brake: 0, steer: 0, handbrake: false };
    return { throttle, brake, steer, handbrake };
  }

  drain(): Press[] {
    const out = this.presses;
    this.presses = [];
    return out;
  }

  dispose() {
    this.target.removeEventListener('keydown', this.keydown);
    this.target.removeEventListener('keyup', this.keyup);
    this.target.removeEventListener('blur', this.blur);
  }
}
