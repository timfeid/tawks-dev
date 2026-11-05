import { describe, expect, it } from 'vitest';
import { HIT, MISSED } from './engine';
import { VocalEngine, hzToMidi, pitchDistance } from './vocals';

const sing = (from: number, to: number, midi: number | null, level = 0.1) =>
  Array.from({ length: Math.floor((to - from) / 16) }, (_, i) => ({ time: from + i * 16, midi, level }));

describe('pitch', () => {
  it('converts hertz and ignores octaves', () => {
    expect(Math.round(hzToMidi(440))).toBe(69);
    expect(Math.round(hzToMidi(261.63))).toBe(60);
    expect(pitchDistance(72, 60)).toBe(0);
    expect(pitchDistance(61.5, 60)).toBeCloseTo(1.5);
    expect(pitchDistance(59, 60)).toBe(1);
  });
});

describe('VocalEngine', () => {
  const notes = [
    { t: 1000, len: 500, pitch: 60, talky: false },
    { t: 2000, len: 500, pitch: 64, talky: false },
    { t: 3000, len: 400, pitch: 0, talky: true },
  ];

  it('scores notes held on pitch and misses ones sung off key', () => {
    const engine = new VocalEngine(notes, 'hard');
    engine.feed(sing(1000, 1500, 72.4));
    engine.feed(sing(2000, 2500, 67));
    engine.feed(sing(3000, 3400, null, 0.05));
    engine.update(4000);
    expect([...engine.judged]).toEqual([HIT, MISSED, HIT]);
    expect(engine.streak).toBe(1);
    expect(engine.progress().total).toBe(3);
  });

  it('is more forgiving on easy and needs half the note', () => {
    const easy = new VocalEngine(notes, 'easy');
    easy.feed(sing(2000, 2300, 67));
    easy.update(4000);
    expect(easy.judged[1]).toBe(HIT);
    const short = new VocalEngine(notes, 'easy');
    short.feed(sing(2000, 2150, 64));
    short.update(4000);
    expect(short.judged[1]).toBe(MISSED);
  });

  it('drops the real vocals live while the singer is off the note and brings them back on it', () => {
    const engine = new VocalEngine(notes, 'hard');
    engine.feed(sing(1000, 1500, 60));
    expect(engine.muted).toBe(false);
    engine.feed(sing(2000, 2400, 70));
    expect(engine.muted).toBe(true);
    expect(engine.mutedAt).toBeGreaterThanOrEqual(2150);
    expect(engine.mutedAt).toBeLessThan(2200);
    engine.feed(sing(2400, 2500, 64));
    expect(engine.muted).toBe(false);
    expect(engine.mutedAt).toBeLessThan(2460);
  });

  it('drops the vocals when the singer goes quiet during a note, not between notes', () => {
    const engine = new VocalEngine(notes, 'hard');
    engine.feed(sing(1000, 1500, 60));
    engine.feed(sing(1600, 1990, null, 0));
    expect(engine.muted).toBe(false);
    engine.feed(sing(2000, 2300, null, 0));
    expect(engine.muted).toBe(true);
  });
});
