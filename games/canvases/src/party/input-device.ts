import { gamepad } from './settings';

/** What the player last touched: keyboard and mouse, or a controller. */
export type InputDevice = 'keys' | 'pad';

let device: InputDevice = 'keys';
const watchers = new Set<(device: InputDevice) => void>();
let started = false;

function use(next: InputDevice) {
  if (next === device) return;
  device = next;
  for (const watcher of watchers) watcher(device);
}

/** Listens once for whichever is used, and keeps listening for the page's life. */
function start() {
  if (started || typeof window === 'undefined') return;
  started = true;
  window.addEventListener('keydown', () => use('keys'), true);
  window.addEventListener('pointerdown', (event) => { if (event.pointerType === 'mouse') use('keys'); }, true);
  window.addEventListener('mousemove', (event) => { if (Math.abs(event.movementX) + Math.abs(event.movementY) > 4) use('keys'); }, true);
  // Controllers send no events while held, so look at them a few times a second.
  setInterval(() => {
    const pad = gamepad();
    if (!pad) return;
    if (pad.buttons.some((button) => button.pressed) || pad.axes.some((axis) => Math.abs(axis) > 0.5)) use('pad');
  }, 200);
}

export function inputDevice(): InputDevice {
  start();
  return device;
}

/** Calls `watcher` now and whenever the player switches; returns the way to stop. */
export function watchInputDevice(watcher: (device: InputDevice) => void) {
  start();
  watchers.add(watcher);
  watcher(device);
  return () => watchers.delete(watcher);
}

/** The standard layout's buttons, by index, as an Xbox-style pad labels them. */
const PAD_BUTTONS = ['A', 'B', 'X', 'Y', 'LB', 'RB', 'LT', 'RT', 'View', 'Menu', 'L3', 'R3', 'D↑', 'D↓', 'D←', 'D→'];

export function padButton(index: number) {
  return PAD_BUTTONS[index] ?? `Button ${index}`;
}
