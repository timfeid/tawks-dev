export type StickValue = { x: number; z: number };

const KEYS: Record<string, [number, number]> = {
  KeyW: [0, -1], ArrowUp: [0, -1],
  KeyS: [0, 1], ArrowDown: [0, 1],
  KeyA: [-1, 0], ArrowLeft: [-1, 0],
  KeyD: [1, 0], ArrowRight: [1, 0],
};

const isTyping = (target: EventTarget | null) =>
  target instanceof HTMLElement && (target.isContentEditable || ['INPUT', 'TEXTAREA', 'SELECT'].includes(target.tagName));

/**
 * Merges keyboard, the first gamepad's left stick and an on-screen joystick
 * into one analog stick. Screen up is away from the camera (-z).
 */
export class PartyInput {
  private held = new Set<string>();
  private touch: StickValue = { x: 0, z: 0 };
  enabled = false;

  private onKeyDown = (event: KeyboardEvent) => {
    if (!this.enabled || !(event.code in KEYS) || isTyping(event.target)) return;
    this.held.add(event.code);
    event.preventDefault();
  };
  private onKeyUp = (event: KeyboardEvent) => this.held.delete(event.code);
  private onBlur = () => this.held.clear();

  constructor() {
    window.addEventListener('keydown', this.onKeyDown);
    window.addEventListener('keyup', this.onKeyUp);
    window.addEventListener('blur', this.onBlur);
  }

  setTouch(value: StickValue) {
    this.touch = value;
  }

  read(): StickValue {
    let x = this.touch.x;
    let z = this.touch.z;
    for (const code of this.held) {
      x += KEYS[code][0];
      z += KEYS[code][1];
    }
    const pad = typeof navigator.getGamepads === 'function' ? Array.from(navigator.getGamepads()).find(Boolean) : null;
    if (pad && pad.axes.length >= 2) {
      const [px, pz] = pad.axes;
      if (Math.hypot(px, pz) > 0.18) {
        x += px;
        z += pz;
      }
    }
    const length = Math.hypot(x, z);
    return length > 1 ? { x: x / length, z: z / length } : { x, z };
  }

  dispose() {
    window.removeEventListener('keydown', this.onKeyDown);
    window.removeEventListener('keyup', this.onKeyUp);
    window.removeEventListener('blur', this.onBlur);
  }
}
