export type Control = 'lane0' | 'lane1' | 'lane2' | 'lane3' | 'lane4' | 'strum_up' | 'strum_down' | 'star';
export type Layout = 'guitar' | 'drums';

export type Binding =
  | { kind: 'key'; code: string }
  | { kind: 'button'; index: number }
  | { kind: 'axis'; index: number; dir: 1 | -1 };

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

const AXIS_THRESHOLD = 0.5;
const POLL_MS = 2;

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

function pads() {
  return [...(navigator.getGamepads?.() ?? [])].filter((p): p is Gamepad => !!p && p.connected);
}

export function connectedPads() {
  return pads().map((p) => p.id.replace(/\s*\(.*?Vendor.*\)$/, ''));
}

function pressed(pad: Gamepad, binding: Binding) {
  if (binding.kind === 'button') return !!pad.buttons[binding.index]?.pressed;
  if (binding.kind === 'axis') return (pad.axes[binding.index] ?? 0) * binding.dir > AXIS_THRESHOLD;
  return false;
}

/**
 * Turns keys and controller buttons into control presses with the time they
 * happened. Pads are polled every couple of milliseconds and stamped with the
 * pad's own report time, so a press is not late by a whole frame.
 */
export class Controls {
  private down = new Map<Control, boolean>();
  private keys = new Set<string>();
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

  private refresh(at: number, connected = pads()) {
    for (const control of Object.keys(this.bindings) as Control[]) {
      const now = this.bindings[control].some((binding) =>
        binding.kind === 'key' ? this.keys.has(binding.code) : connected.some((pad) => pressed(pad, binding)),
      );
      if (now !== (this.down.get(control) ?? false)) {
        this.down.set(control, now);
        this.emit(control, now, at);
      }
    }
  }
}

/**
 * Waits for the next key, button or stick push, for binding a control. Escape
 * cancels. With `padsOnly`, keys other than Escape are ignored.
 */
export function captureBinding(signal: AbortSignal, padsOnly = false): Promise<Binding | null> {
  return new Promise((resolve) => {
    const rest = new Map(pads().map((p) => [p.index, { buttons: p.buttons.map((b) => b.pressed), axes: [...p.axes] }]));
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
      for (const pad of pads()) {
        const before = rest.get(pad.index) ?? { buttons: [], axes: [] };
        const buttonIndex = pad.buttons.findIndex((b, i) => b.pressed && !before.buttons[i]);
        if (buttonIndex >= 0) return finish({ kind: 'button', index: buttonIndex });
        const axisIndex = pad.axes.findIndex((v, i) => Math.abs(v - (before.axes[i] ?? 0)) > 0.6 && Math.abs(v) > AXIS_THRESHOLD);
        if (axisIndex >= 0) return finish({ kind: 'axis', index: axisIndex, dir: pad.axes[axisIndex] > 0 ? 1 : -1 });
        if (!rest.has(pad.index)) rest.set(pad.index, { buttons: pad.buttons.map((b) => b.pressed), axes: [...pad.axes] });
      }
    }, 10);
    window.addEventListener('keydown', onKey, { capture: true });
    signal.addEventListener('abort', () => finish(null));
  });
}
