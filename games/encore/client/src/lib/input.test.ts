import { afterEach, describe, expect, it, vi } from 'vitest';
import {
  AXIS_HYSTERESIS,
  Controls,
  DEFAULT_BINDINGS,
  axisEngaged,
  captureThreshold,
  cleanBindings,
  type AxisBinding,
  type Bindings,
  type Control,
} from './input';

const tilt: AxisBinding = { kind: 'axis', index: 2, dir: 1, threshold: 0.4, pad: 'Guitar' };

describe('axisEngaged', () => {
  it('presses past the threshold and lets go only under it by the hysteresis', () => {
    expect(axisEngaged(0.39, tilt, false)).toBe(false);
    expect(axisEngaged(0.41, tilt, false)).toBe(true);
    expect(axisEngaged(0.3, tilt, true)).toBe(true);
    expect(axisEngaged(0.4 - AXIS_HYSTERESIS - 0.01, tilt, true)).toBe(false);
    expect(axisEngaged(-0.6, { ...tilt, dir: -1 }, false)).toBe(true);
    expect(axisEngaged(0.6, { kind: 'axis', index: 0, dir: 1 }, false)).toBe(true);
  });
});

describe('captureThreshold', () => {
  it('sits halfway between rest and how far the axis went', () => {
    expect(captureThreshold(0, 1, 1)).toBe(0.5);
    expect(captureThreshold(-1, 1, 1)).toBe(0);
    expect(captureThreshold(0, -1, -1)).toBe(0.5);
    expect(captureThreshold(0.98, 1, 1)).toBe(0.95);
  });
});

describe('cleanBindings', () => {
  it('keeps well formed bindings and clamps axis thresholds', () => {
    expect(cleanBindings('x')).toBeNull();
    expect(
      cleanBindings([
        { kind: 'key', code: 'Space' },
        { kind: 'button', index: 8 },
        { kind: 'axis', index: 2, dir: 1, threshold: 4, pad: 'Guitar' },
        { kind: 'axis', index: 2, dir: 0 },
        null,
      ]),
    ).toEqual([
      { kind: 'key', code: 'Space' },
      { kind: 'button', index: 8 },
      { kind: 'axis', index: 2, dir: 1, threshold: 0.95, pad: 'Guitar' },
    ]);
  });
});

describe('Controls', () => {
  afterEach(() => {
    vi.unstubAllGlobals();
    vi.useRealTimers();
  });

  it('fires star power once per tilt, not for every wobble while tilted', () => {
    vi.useFakeTimers();
    const pad = { id: 'Guitar', index: 0, connected: true, timestamp: 0, buttons: [] as { pressed: boolean }[], axes: [0, 0, -1] };
    const other = { ...pad, id: 'Stick', index: 1, axes: [0, 0, 1] };
    vi.stubGlobal('navigator', { getGamepads: () => [pad, other] });
    vi.stubGlobal('window', { addEventListener() {}, removeEventListener() {} });
    const bindings: Bindings = { ...DEFAULT_BINDINGS.guitar, star: [tilt] };
    const events: [Control, boolean][] = [];
    const controls = new Controls(bindings, (control, down) => events.push([control, down]));
    controls.start();
    const step = (value: number) => {
      pad.axes[2] = value;
      pad.timestamp++;
      vi.advanceTimersByTime(4);
    };
    for (const value of [0.2, 0.45, 0.38, 0.42, 0.3, 0.5, 0.1, 0.6]) step(value);
    controls.stop();
    expect(events.filter(([c]) => c === 'star')).toEqual([
      ['star', true],
      ['star', false],
      ['star', true],
    ]);
  });
});
