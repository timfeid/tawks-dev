/**
 * A player's own settings for the party game: key bindings, the controller,
 * sound and the camera. Kept in this browser; nothing here is shared with the
 * room. Everything that reads input, plays sound or moves the camera asks
 * here, so a change takes hold straight away.
 */

/** Everything a key can be bound to. */
export type Action = 'up' | 'down' | 'left' | 'right' | 'action';

/** Controller buttons, by their standard-mapping index. */
export const PAD_BUTTONS: Record<number, string> = {
  0: 'A / Cross',
  1: 'B / Circle',
  2: 'X / Square',
  3: 'Y / Triangle',
  4: 'Left bumper',
  5: 'Right bumper',
  6: 'Left trigger',
  7: 'Right trigger',
};

export type Settings = {
  keys: Record<Action, string[]>;
  pad: { action: number; deadzone: number };
  sound: { master: number; effects: number; jingles: number; ambience:number };
  camera: { sensitivity: number; invertY: boolean };
};

export const DEFAULTS: Settings = {
  keys: {
    up: ['KeyW', 'ArrowUp'],
    down: ['KeyS', 'ArrowDown'],
    left: ['KeyA', 'ArrowLeft'],
    right: ['KeyD', 'ArrowRight'],
    action: ['Space'],
  },
  pad: { action: 0, deadzone: 0.18 },
  sound: { master: 0.9, effects: 1, jingles: 0.8, ambience:.7 },
  camera: { sensitivity: 1, invertY: false },
};

const STORE = 'party-settings';

function load(): Settings {
  try {
    const saved = JSON.parse(localStorage.getItem(STORE) ?? 'null');
    if (!saved || typeof saved !== 'object') return structuredClone(DEFAULTS);
    return {
      keys: { ...DEFAULTS.keys, ...saved.keys },
      pad: { ...DEFAULTS.pad, ...saved.pad },
      sound: { ...DEFAULTS.sound, ...saved.sound },
      camera: { ...DEFAULTS.camera, ...saved.camera },
    };
  } catch {
    return structuredClone(DEFAULTS);
  }
}

let current = load();
const listeners = new Set<(settings: Settings) => void>();

export function settings(): Settings {
  return current;
}

export function updateSettings(change: (draft: Settings) => void) {
  const next = structuredClone(current);
  change(next);
  current = next;
  try {
    localStorage.setItem(STORE, JSON.stringify(current));
  } catch {
    // Remembering them is a convenience only.
  }
  for (const listener of listeners) listener(current);
}

export function resetSettings() {
  updateSettings((draft) => Object.assign(draft, structuredClone(DEFAULTS)));
}

/** Calls back on every change, and once now. Returns the way to stop. */
export function watchSettings(listener: (settings: Settings) => void) {
  listeners.add(listener);
  listener(current);
  return () => listeners.delete(listener);
}

/** Whether a key is bound to an action. */
export function bound(action: Action, code: string) {
  return current.keys[action].includes(code);
}

/** A key's name as it reads on the keyboard. */
export function keyName(code: string) {
  if (code.startsWith('Key')) return code.slice(3);
  if (code.startsWith('Digit')) return code.slice(5);
  const names: Record<string, string> = {
    Space: 'Space',
    ShiftLeft: 'Left Shift',
    ShiftRight: 'Right Shift',
    ControlLeft: 'Left Ctrl',
    ControlRight: 'Right Ctrl',
    AltLeft: 'Left Alt',
    AltRight: 'Right Alt',
    ArrowUp: '↑',
    ArrowDown: '↓',
    ArrowLeft: '←',
    ArrowRight: '→',
    Enter: 'Enter',
    Tab: 'Tab',
  };
  return names[code] ?? code;
}

/** The first connected controller, if any. */
export function gamepad(): Gamepad | null {
  if (typeof navigator.getGamepads !== 'function') return null;
  return Array.from(navigator.getGamepads()).find((pad): pad is Gamepad => !!pad) ?? null;
}
