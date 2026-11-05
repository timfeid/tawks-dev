export type Control = 'lane0' | 'lane1' | 'lane2' | 'lane3' | 'lane4' | 'strum_up' | 'strum_down' | 'star';
export type Layout = 'guitar' | 'drums';

export type Binding =
  | { kind: 'key'; code: string }
  | { kind: 'button'; index: number }
  /**
   * `threshold` is how far along `dir` the axis must go to press, from -1 to
   * 1; `pad` is the controller id it was bound on, so a tilt sensor's axis
   * number on one guitar does not fire a stick on another controller.
   */
  | { kind: 'axis'; index: number; dir: 1 | -1; threshold?: number; pad?: string };

export type AxisBinding = Extract<Binding, { kind: 'axis' }>;

export type Bindings = Record<Control, Binding[]>;

export const CONTROLS: Record<Layout, Control[]> = {
  guitar: ['lane0', 'lane1', 'lane2', 'lane3', 'lane4', 'strum_down', 'strum_up', 'star'],
  drums: ['lane0', 'lane1', 'lane2', 'lane3', 'lane4', 'star'],
};

export const CONTROL_LABELS: Record<Layout, Partial<Record<Control, string>>> = {
  guitar: {
    lane0: 'Green fret',
    lane1: 'Red fret',
    lane2: 'Yellow fret',
    lane3: 'Blue fret',
    lane4: 'Orange fret',
    strum_down: 'Strum down',
    strum_up: 'Strum up',
    star: 'Star power',
  },
  drums: {
    lane0: 'Kick pedal',
    lane1: 'Red pad',
    lane2: 'Yellow pad',
    lane3: 'Blue pad',
    lane4: 'Green pad',
    star: 'Star power',
  },
};

const key = (code: string): Binding => ({ kind: 'key', code });
const button = (index: number): Binding => ({ kind: 'button', index });

/** Xbox-style guitars and drum kits report the standard face buttons, so these work for most out of the box. */
export const DEFAULT_BINDINGS: Record<Layout, Bindings> = {
  guitar: {
    lane0: [key('KeyA'), button(0)],
    lane1: [key('KeyS'), button(1)],
    lane2: [key('KeyD'), button(3)],
    lane3: [key('KeyF'), button(2)],
    lane4: [key('KeyG'), button(4)],
    strum_down: [key('ArrowDown'), key('Enter'), button(13)],
    strum_up: [key('ArrowUp'), button(12)],
    star: [key('Space'), button(8)],
  },
  drums: {
    lane0: [key('Space'), button(4)],
    lane1: [key('KeyJ'), button(1)],
    lane2: [key('KeyK'), button(3)],
    lane3: [key('KeyL'), button(2)],
    lane4: [key('Semicolon'), button(0)],
    strum_down: [],
    strum_up: [],
    star: [key('Enter'), button(8)],
  },
};

export const AXIS_THRESHOLD = 0.5;
/** A pressed axis lets go this far short of its threshold, so a tilt held near the edge does not flicker. */
export const AXIS_HYSTERESIS = 0.15;
export const MIN_AXIS_THRESHOLD = -0.9;
export const MAX_AXIS_THRESHOLD = 0.95;
const POLL_MS = 2;
/** How far an axis must move from rest to be bound. */
const CAPTURE_MOVE = 0.5;
/** How long capture keeps watching a moved axis for how far it goes. */
const CAPTURE_SETTLE_MS = 300;

/** How far along its direction an axis binding reads, from -1 to 1. */
export function axisAlong(value: number, binding: AxisBinding) {
  return value * binding.dir;
}

/** Whether an axis counts as pressed: it presses past the threshold and lets go only once back under it by the hysteresis. */
export function axisEngaged(value: number, binding: AxisBinding, wasEngaged: boolean) {
  const threshold = binding.threshold ?? AXIS_THRESHOLD;
  const along = axisAlong(value, binding);
  return wasEngaged ? along > threshold - AXIS_HYSTERESIS : along > threshold;
}

/** The threshold halfway between where an axis rested and how far it went when bound. */
export function captureThreshold(rest: number, peak: number, dir: 1 | -1) {
  const middle = ((rest + peak) / 2) * dir;
  return Math.round(Math.min(MAX_AXIS_THRESHOLD, Math.max(MIN_AXIS_THRESHOLD, middle)) * 100) / 100;
}

/** Keeps only well-formed bindings from saved settings. */
export function cleanBindings(raw: unknown): Binding[] | null {
  if (!Array.isArray(raw)) return null;
  const out: Binding[] = [];
  for (const b of raw as Record<string, unknown>[]) {
    if (!b || typeof b !== 'object') continue;
    if (b.kind === 'key' && typeof b.code === 'string') out.push({ kind: 'key', code: b.code });
    else if (b.kind === 'button' && Number.isInteger(b.index)) out.push({ kind: 'button', index: b.index as number });
    else if (b.kind === 'axis' && Number.isInteger(b.index) && (b.dir === 1 || b.dir === -1)) {
      const axis: AxisBinding = { kind: 'axis', index: b.index as number, dir: b.dir };
      if (typeof b.threshold === 'number' && Number.isFinite(b.threshold)) {
        axis.threshold = Math.min(MAX_AXIS_THRESHOLD, Math.max(MIN_AXIS_THRESHOLD, b.threshold));
      }
      if (typeof b.pad === 'string' && b.pad) axis.pad = b.pad.slice(0, 200);
      out.push(axis);
    }
  }
  return out;
}

export function describeBinding(binding: Binding) {
  switch (binding.kind) {
    case 'key':
      return binding.code.replace(/^Key/, '').replace(/^Digit/, '').replace(/^Arrow/, 'Arrow ');
    case 'button':
      return `Button ${binding.index + 1}`;
    case 'axis':
      return `Axis ${binding.index + 1} ${binding.dir > 0 ? '+' : '-'}`;
  }
}

export function padName(id: string) {
  return id.replace(/\s*\(.*?Vendor.*\)$/, '');
}

function pads() {
  return [...(navigator.getGamepads?.() ?? [])].filter((p): p is Gamepad => !!p && p.connected);
}

export function connectedPads() {
  return pads().map((p) => padName(p.id));
}

/** The current reading of an axis binding along its direction, from the controller it was bound on, or null if none is connected. */
export function readAxis(binding: AxisBinding): number | null {
  for (const pad of pads()) {
    if (binding.pad && pad.id !== binding.pad) continue;
    const value = pad.axes[binding.index];
    if (value !== undefined) return axisAlong(value, binding);
  }
  return null;
}

/**
 * Turns keys and controller buttons into control presses with the time they
 * happened. Pads are polled every couple of milliseconds and stamped with the
 * pad's own report time, so a press is not late by a whole frame.
 */
export class Controls {
  private down = new Map<Control, boolean>();
  private keys = new Set<string>();
  /** Which axis bindings are pressed, by control, binding and pad, for their hysteresis. */
  private axes = new Map<string, boolean>();
  private timer: ReturnType<typeof setInterval> | null = null;

  constructor(
    private bindings: Bindings,
    private emit: (control: Control, down: boolean, at: number) => void,
  ) {}

  start() {
    window.addEventListener('keydown', this.onKey, { capture: true });
    window.addEventListener('keyup', this.onKey, { capture: true });
    window.addEventListener('blur', this.onBlur);
    this.timer = setInterval(() => this.poll(), POLL_MS);
  }

  stop() {
    window.removeEventListener('keydown', this.onKey, { capture: true });
    window.removeEventListener('keyup', this.onKey, { capture: true });
    window.removeEventListener('blur', this.onBlur);
    if (this.timer) clearInterval(this.timer);
  }

  private onKey = (event: KeyboardEvent) => {
    if (event.repeat) return;
    const bound = (Object.keys(this.bindings) as Control[]).some((c) =>
      this.bindings[c].some((b) => b.kind === 'key' && b.code === event.code),
    );
    if (!bound) return;
    event.preventDefault();
    if (event.type === 'keydown') this.keys.add(event.code);
    else this.keys.delete(event.code);
    this.refresh(event.timeStamp);
  };

  private onBlur = () => {
    this.keys.clear();
    this.refresh(performance.now());
  };

  private poll() {
    const connected = pads();
    if (!connected.length && !this.keys.size && ![...this.down.values()].some(Boolean)) return;
    const at = connected.reduce((latest, p) => Math.max(latest, p.timestamp), 0) || performance.now();
    this.refresh(Math.min(at, performance.now()), connected);
  }

  private pressed(control: Control, slot: number, pad: Gamepad, binding: Binding) {
    if (binding.kind === 'button') return !!pad.buttons[binding.index]?.pressed;
    if (binding.kind !== 'axis') return false;
    if (binding.pad && pad.id !== binding.pad) return false;
    const key = `${control}:${slot}:${pad.index}`;
    const engaged = axisEngaged(pad.axes[binding.index] ?? 0, binding, this.axes.get(key) ?? false);
    this.axes.set(key, engaged);
    return engaged;
  }

  private refresh(at: number, connected = pads()) {
    for (const control of Object.keys(this.bindings) as Control[]) {
      let now = false;
      this.bindings[control].forEach((binding, slot) => {
        if (binding.kind === 'key') now ||= this.keys.has(binding.code);
        else for (const pad of connected) now = this.pressed(control, slot, pad, binding) || now;
      });
      if (now !== (this.down.get(control) ?? false)) {
        this.down.set(control, now);
        this.emit(control, now, at);
      }
    }
  }
}

/**
 * Waits for the next key, button or axis push, for binding a control. Escape
 * cancels. With `padsOnly`, keys other than Escape are ignored. An axis is
 * watched a moment longer to see how far it goes, and binds on that
 * controller with its threshold halfway there.
 */
export function captureBinding(signal: AbortSignal, padsOnly = false): Promise<Binding | null> {
  return new Promise((resolve) => {
    const rest = new Map(pads().map((p) => [p.index, { buttons: p.buttons.map((b) => b.pressed), axes: [...p.axes] }]));
    let moving: { pad: number; id: string; index: number; rest: number; peak: number; dir: 1 | -1; since: number } | null = null;
    const finish = (binding: Binding | null) => {
      clearInterval(timer);
      window.removeEventListener('keydown', onKey, { capture: true });
      resolve(binding);
    };
    const onKey = (event: KeyboardEvent) => {
      if (padsOnly && event.code !== 'Escape') return;
      event.preventDefault();
      event.stopPropagation();
      finish(event.code === 'Escape' ? null : { kind: 'key', code: event.code });
    };
    const timer = setInterval(() => {
      const now = performance.now();
      for (const pad of pads()) {
        const before = rest.get(pad.index);
        if (!before) {
          rest.set(pad.index, { buttons: pad.buttons.map((b) => b.pressed), axes: [...pad.axes] });
          continue;
        }
        if (moving) {
          if (moving.pad !== pad.index) continue;
          const value = pad.axes[moving.index] ?? 0;
          if ((value - moving.peak) * moving.dir > 0) moving.peak = value;
          if (now - moving.since >= CAPTURE_SETTLE_MS) {
            return finish({
              kind: 'axis',
              index: moving.index,
              dir: moving.dir,
              threshold: captureThreshold(moving.rest, moving.peak, moving.dir),
              pad: moving.id,
            });
          }
          continue;
        }
        const buttonIndex = pad.buttons.findIndex((b, i) => b.pressed && !before.buttons[i]);
        if (buttonIndex >= 0) return finish({ kind: 'button', index: buttonIndex });
        const axisIndex = pad.axes.findIndex((v, i) => Math.abs(v - (before.axes[i] ?? 0)) > CAPTURE_MOVE);
        if (axisIndex >= 0) {
          const from = before.axes[axisIndex] ?? 0;
          const value = pad.axes[axisIndex];
          moving = { pad: pad.index, id: pad.id, index: axisIndex, rest: from, peak: value, dir: value > from ? 1 : -1, since: now };
        }
      }
    }, 10);
    window.addEventListener('keydown', onKey, { capture: true });
    signal.addEventListener('abort', () => finish(null));
  });
}
