import { gamepad, settings } from './settings';
import {isGameplayTyping}from'./typing';

export type StickValue = { x: number; z: number };

/** Which way each key pushes the stick, from the player's bindings. */
function direction(code: string): [number, number] | null {
  const { keys } = settings();
  if (keys.up.includes(code)) return [0, -1];
  if (keys.down.includes(code)) return [0, 1];
  if (keys.left.includes(code)) return [-1, 0];
  if (keys.right.includes(code)) return [1, 0];
  return null;
}

/**
 * Merges keyboard, the first gamepad's left stick and an on-screen joystick
 * into one analog stick. Screen up is away from the camera (-z).
 */
export class PartyInput {
  private held = new Set<string>();
  private touch: StickValue = { x: 0, z: 0 };
  enabled = false;

  private onKeyDown = (event: KeyboardEvent) => {
    if(event.isComposing||isGameplayTyping(event.target)){this.clear();return;}
    if (!this.enabled || !direction(event.code)) return;
    this.held.add(event.code);
    event.preventDefault();
  };
  private onKeyUp = (event: KeyboardEvent) => this.held.delete(event.code);
  private clear(){this.held.clear();this.touch={x:0,z:0};}
  private onBlur = () => this.clear();
  private onFocusIn=(event:FocusEvent)=>{if(isGameplayTyping(event.target))this.clear();};

  constructor() {
    window.addEventListener('keydown', this.onKeyDown);
    window.addEventListener('keyup', this.onKeyUp);
    window.addEventListener('blur', this.onBlur);
    document.addEventListener('focusin',this.onFocusIn);
  }

  setTouch(value: StickValue) {
    if(isGameplayTyping()){this.clear();return;}
    this.touch = value;
  }

  read(): StickValue {
    if(!this.enabled||isGameplayTyping()){this.clear();return {x:0,z:0};}
    let x = this.touch.x;
    let z = this.touch.z;
    for (const code of this.held) {
      const [dx, dz] = direction(code) ?? [0, 0];
      x += dx;
      z += dz;
    }
    const pad = gamepad();
    if (pad && pad.axes.length >= 2) {
      const [px, pz] = pad.axes;
      if (Math.hypot(px, pz) > settings().pad.deadzone) {
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
    document.removeEventListener('focusin',this.onFocusIn);
    this.clear();
  }
}
