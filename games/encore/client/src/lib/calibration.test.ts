import { describe, expect, it } from 'vitest';
import { Engine, HIT } from './engine';
import {
  MAX_AUTO_STEP_MS,
  autoStep,
  calibrationResult,
  playSuggestion,
  robustMedian,
  videoResult,
} from './calibration';
import type { Note } from './types';

describe('robustMedian', () => {
  it('ignores slips far from the rest', () => {
    const taps = [30, 32, 28, 31, 29, 33, 30, 27, 250, -180, 31];
    expect(robustMedian(taps)).toEqual({ median: 30, kept: 9 });
  });

  it('keeps steady taps a few milliseconds apart', () => {
    expect(robustMedian([10, 10, 10, 10, 12, 20])?.kept).toBe(6);
    expect(robustMedian([])).toBeNull();
  });
});

describe('calibrationResult', () => {
  it('needs enough good taps', () => {
    expect(calibrationResult([40, 41, 39])).toBeNull();
    expect(calibrationResult([40, 41, 39, 42, 38, 40, 41, 39, 300])).toBe(40);
  });
});

describe('playSuggestion', () => {
  const around = (center: number, n = 60) => Array.from({ length: n }, (_, i) => center + ((i * 7) % 11) - 5);

  it('speaks up only past the threshold with enough hits', () => {
    expect(playSuggestion(around(21))).toBe(21);
    expect(playSuggestion(around(-15))).toBe(-15);
    expect(playSuggestion(around(6))).toBeNull();
    expect(playSuggestion(around(30, 40))).toBeNull();
  });
});

describe('autoStep', () => {
  it('moves half the error each song, never more than the cap', () => {
    expect(autoStep(0, 12)).toBe(6);
    expect(autoStep(20, -40)).toBe(20 - MAX_AUTO_STEP_MS);
    expect(autoStep(5, 100)).toBe(5 + MAX_AUTO_STEP_MS);
    let offset = 0;
    for (let song = 0; song < 8; song++) offset = autoStep(offset, 35 - offset);
    expect(offset).toBeGreaterThanOrEqual(34);
    expect(offset).toBeLessThanOrEqual(35);
  });
});

describe('videoResult', () => {
  it('moves the notes by how late the taps landed', () => {
    expect(videoResult(0, Array(10).fill(25))).toBe(-25);
    expect(videoResult(-25, Array(10).fill(0))).toBe(-25);
  });
});

describe('calibrated judging', () => {
  it('lands taps that trail the click by a fixed delay right on time once calibrated', () => {
    const delay = 37;
    const jitter = (i: number) => ((i * 5) % 7) - 3;
    const clicks = Array.from({ length: 24 }, (_, i) => 1000 + i * 667);
    const deviations = clicks.map((t, i) => t + delay + jitter(i) - t);
    deviations[5] = 220;
    const offset = calibrationResult(deviations)!;
    expect(offset).toBe(delay);

    const notes: Note[] = clicks.map((t) => ({ t, len: 0, lanes: 1, kind: 'strum' }));
    const engine = new Engine({ instrument: 'guitar', difficulty: 'expert', notes, star_power: [] }, [{ t: 0 }, { t: 500 }]);
    engine.input({ action: 'lane0', down: true, time: 0 });
    clicks.forEach((t, i) => engine.input({ action: 'strum', down: true, time: t + delay + jitter(i) - offset }));
    expect([...engine.judged].every((j) => j === HIT)).toBe(true);
    expect(Math.max(...engine.hitErrors.map(Math.abs))).toBeLessThanOrEqual(3);
  });
});
