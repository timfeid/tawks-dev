import { describe, expect, it } from 'vitest';
import { Engine, HIT, MISSED, PENDING, fretsMatch, type Action } from './engine';
import { OPEN, type Note, type Track } from './types';

const beats = Array.from({ length: 200 }, (_, i) => ({ t: i * 500 }));

function track(notes: Note[], instrument: Track['instrument'] = 'guitar', star_power: [number, number][] = []): Track {
  return { instrument, difficulty: 'expert', notes, star_power };
}

const note = (t: number, lanes: number, kind: Note['kind'] = 'strum', len = 0): Note => ({ t, len, lanes, kind });

function press(engine: Engine, action: Action, time: number, down = true) {
  engine.input({ action, down, time });
}

describe('fretsMatch', () => {
  it('lets lower frets anchor a single note but wants chords exact', () => {
    expect(fretsMatch(0b011, 0b010)).toBe(true);
    expect(fretsMatch(0b110, 0b010)).toBe(false);
    expect(fretsMatch(0b011, 0b011)).toBe(true);
    expect(fretsMatch(0b111, 0b011)).toBe(false);
    expect(fretsMatch(0, OPEN)).toBe(true);
    expect(fretsMatch(1, OPEN)).toBe(false);
  });
});

describe('Engine', () => {
  it('hits strummed notes inside the window and misses the rest', () => {
    const engine = new Engine(track([note(1000, 1), note(2000, 2), note(3000, 4)]), beats);
    press(engine, 'lane0', 900);
    press(engine, 'strum', 1040);
    press(engine, 'lane0', 1500, false);
    press(engine, 'lane1', 1500);
    press(engine, 'strum', 1900);
    engine.update(3200);
    expect([...engine.judged]).toEqual([HIT, MISSED, MISSED]);
    expect(engine.missFlashes[2]).toBeGreaterThan(3000);
    expect(engine.score).toBe(50);
    expect(engine.streak).toBe(0);
    expect(engine.muted).toBe(true);
  });

  it('breaks the streak on a strum with nothing to hit', () => {
    const engine = new Engine(track([note(1000, 1), note(2000, 1)]), beats);
    press(engine, 'lane0', 0);
    press(engine, 'strum', 1000);
    expect(engine.streak).toBe(1);
    press(engine, 'strum', 1500);
    expect(engine.streak).toBe(0);
    expect(engine.overstrumAt).toBe(1500);
    expect(engine.judged[1]).toBe(PENDING);
  });

  it('plays hammer-ons by fretting once the streak is going, and ignores the strum through them', () => {
    const engine = new Engine(track([note(1000, 1), note(1100, 2, 'hopo'), note(1200, 4, 'hopo')]), beats);
    press(engine, 'lane0', 990);
    press(engine, 'strum', 1000);
    press(engine, 'lane1', 1100);
    press(engine, 'strum', 1120);
    expect(engine.streak).toBe(2);
    press(engine, 'lane2', 1195);
    expect([...engine.judged]).toEqual([HIT, HIT, HIT]);
  });

  it('needs a strum for a hammer-on after a miss, but never for a tap', () => {
    const engine = new Engine(track([note(1000, 2, 'hopo'), note(2000, 4, 'tap')]), beats);
    press(engine, 'lane1', 1000);
    expect(engine.judged[0]).toBe(PENDING);
    press(engine, 'strum', 1010);
    expect(engine.judged[0]).toBe(HIT);
    engine.update(1500);
    press(engine, 'lane2', 2000);
    expect(engine.judged[1]).toBe(HIT);
  });

  it('scores sustains while the fret is held and multiplies after ten notes', () => {
    const notes = Array.from({ length: 10 }, (_, i) => note(1000 + i * 100, 1));
    notes.push(note(3000, 1, 'strum', 1000));
    const engine = new Engine(track(notes), beats);
    press(engine, 'lane0', 900);
    for (let i = 0; i < 10; i++) press(engine, 'strum', 1000 + i * 100);
    expect(engine.multiplier).toBe(2);
    press(engine, 'strum', 3000);
    expect(engine.score).toBe(500 + 100);
    engine.update(4000);
    expect(engine.score).toBe(600 + 2 * 25 * 2);
  });

  it('fills star power from clean phrases and doubles the multiplier when used', () => {
    const notes = [note(1000, 1), note(1500, 1), note(2000, 1), note(2500, 1), note(6000, 1)];
    const engine = new Engine(track(notes, 'guitar', [[900, 1600], [1900, 2600]]), beats);
    press(engine, 'lane0', 0);
    for (const t of [1000, 1500, 2000, 2500]) press(engine, 'strum', t);
    expect(engine.starMeter).toBe(0.5);
    press(engine, 'star', 3000);
    expect(engine.starActive).toBe(true);
    expect(engine.multiplier).toBe(2);
    engine.update(3000 + 16 * 500 + 10);
    expect(engine.starActive).toBe(false);
  });

  it('judges each drum gem by its own pad and forgives extra hits', () => {
    const engine = new Engine(track([note(1000, 0b11), note(2000, 0b100)], 'drums'), beats);
    press(engine, 'lane0', 1000);
    press(engine, 'lane1', 1010);
    press(engine, 'lane3', 1500);
    expect(engine.streak).toBe(2);
    engine.update(2200);
    expect([...engine.judged]).toEqual([HIT, HIT, MISSED]);
    expect(engine.progress().total).toBe(3);
  });
});
