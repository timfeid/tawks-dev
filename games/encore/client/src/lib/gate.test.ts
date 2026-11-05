import { describe, expect, it } from 'vitest';
import { CUE_HOLD_MS, SingingGate, StemGate } from './gate';
import type { Instrument } from './types';

function gate() {
  const applied: [Instrument, boolean, number][] = [];
  return { gate: new StemGate((i, out, at) => applied.push([i, out, at])), applied };
}

describe('StemGate', () => {
  it('schedules a remote cue at its song time plus the hold, not when it arrives', () => {
    const { gate: g, applied } = gate();
    g.roster([{ conn: 2, instrument: 'drums', out: false }], 0);
    g.cue(2, 5000, true, CUE_HOLD_MS);
    g.cue(2, 6200, false, CUE_HOLD_MS);
    expect(applied).toEqual([
      ['drums', true, 5000 + CUE_HOLD_MS],
      ['drums', false, 6200 + CUE_HOLD_MS],
    ]);
  });

  it('drops a shared part only while every player on it is out', () => {
    const { gate: g, applied } = gate();
    g.roster(
      [
        { conn: 1, instrument: 'guitar', out: false },
        { conn: 2, instrument: 'guitar', out: false },
      ],
      0,
    );
    g.cue(1, 1000, true, 0);
    expect(applied).toEqual([]);
    g.cue(2, 1400, true, 0);
    expect(applied).toEqual([['guitar', true, 1400]]);
    g.cue(1, 1800, false, 0);
    expect(applied).toEqual([
      ['guitar', true, 1400],
      ['guitar', false, 1800],
    ]);
  });

  it('ignores stale cues and falls back to progress until a player cues', () => {
    const { gate: g, applied } = gate();
    g.roster([{ conn: 3, instrument: 'bass', out: false }], 0);
    g.roster([{ conn: 3, instrument: 'bass', out: true }], 900);
    expect(applied).toEqual([['bass', true, 900]]);
    g.cue(3, 2000, false, 0);
    g.cue(3, 1500, true, 0);
    g.roster([{ conn: 3, instrument: 'bass', out: true }], 2500);
    expect(applied).toEqual([
      ['bass', true, 900],
      ['bass', false, 2000],
    ]);
    g.roster([], 3000);
    g.roster([{ conn: 4, instrument: 'bass', out: false }], 3000);
    expect(g.isOut('bass')).toBe(false);
  });
});

describe('SingingGate', () => {
  it('opens after a short run on pitch, closes after a longer run off it, and holds between notes', () => {
    const s = new SingingGate();
    for (let t = 0; t < 140; t += 10) s.sample(t, false);
    expect(s.out).toBe(false);
    s.sample(150, false);
    expect(s.out).toBe(true);
    expect(s.changedAt).toBe(150);
    s.sample(160, true);
    s.sample(170, null);
    s.sample(180, true);
    expect(s.out).toBe(true);
    for (let t = 190; t <= 230; t += 10) s.sample(t, true);
    expect(s.out).toBe(false);
    expect(s.changedAt).toBe(220);
  });
});
