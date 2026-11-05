import { describe, expect, it } from 'vitest';
import { EARLY_MS, Engine, HIT, MISSED, PENDING, fretsMatch, type Action } from './engine';
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

  it('lets hammer-on and tap chords anchor below their lowest fret', () => {
    expect(fretsMatch(0b00111, 0b00110, 'strum')).toBe(false);
    expect(fretsMatch(0b00111, 0b00110, 'hopo')).toBe(true);
    expect(fretsMatch(0b01001, 0b00110, 'tap')).toBe(false);
    expect(fretsMatch(0b00010, 0b00110, 'hopo')).toBe(false);
    expect(fretsMatch(0b00011, OPEN | 0b10, 'hopo')).toBe(false);
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
    engine.update(1530);
    expect(engine.streak).toBe(0);
    expect(engine.overstrumAt).toBe(1525);
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

  it('lights every press, note or not, on frets and pads', () => {
    const guitar = new Engine(track([note(1000, 1)]), beats);
    press(guitar, 'lane3', 400);
    expect(guitar.presses[3]).toBe(400);
    expect(guitar.flashes[3]).toBeLessThan(0);
    const drums = new Engine(track([note(1000, 0b10)], 'drums'), beats);
    press(drums, 'lane4', 500);
    press(drums, 'lane0', 600);
    expect(drums.presses[4]).toBe(500);
    expect(drums.presses[0]).toBe(600);
    expect(drums.streak).toBe(0);
    press(drums, 'lane1', 1005);
    expect(drums.flashes[1]).toBe(1005);
    expect(drums.judged[0]).toBe(HIT);
  });

  it('marks when the part drops out and comes back', () => {
    const engine = new Engine(track([note(1000, 1), note(2000, 1)]), beats);
    engine.update(1200);
    expect(engine.muted).toBe(true);
    expect(engine.mutedAt).toBeCloseTo(1070, 1);
    press(engine, 'lane0', 1900);
    press(engine, 'strum', 2010);
    expect(engine.muted).toBe(false);
    expect(engine.mutedAt).toBe(2010);
  });

  it('plays kick notes itself with auto kick and ignores the pedal', () => {
    const notes = [note(1000, 0b11), note(1500, 0b1), note(2000, 0b100)];
    const engine = new Engine(track(notes, 'drums'), beats, { autoKick: true });
    press(engine, 'lane0', 400);
    expect(engine.presses[0]).toBeLessThan(0);
    engine.update(1000);
    expect(engine.judged[0]).toBe(HIT);
    expect(engine.flashes[0]).toBe(1000);
    press(engine, 'lane1', 1030);
    engine.update(2200);
    expect([...engine.judged]).toEqual([HIT, HIT, HIT, MISSED]);
    expect(engine.isAutoKick(2)).toBe(true);
    expect(engine.isAutoKick(1)).toBe(false);
    expect(engine.progress().auto_kick).toBe(true);
    expect(new Engine(track(notes, 'guitar'), beats, { autoKick: true }).autoKick).toBe(false);
  });

  it('plays a lower note over an extended sustain without dropping it', () => {
    const engine = new Engine(track([note(1000, 0b100, 'strum', 1000), note(1500, 0b001)]), beats);
    press(engine, 'lane2', 990);
    press(engine, 'strum', 1000);
    press(engine, 'lane0', 1490);
    press(engine, 'strum', 1500);
    expect([...engine.judged]).toEqual([HIT, HIT]);
    expect(engine.streak).toBe(2);
    expect(engine.overstrumAt).toBeLessThan(0);
    expect(engine.isSustaining(0)).toBe(true);
    engine.update(2000);
    expect(engine.score).toBe(100 + 2 * 25);
  });

  it('plays a chord and a hammer-on over an extended sustain', () => {
    const notes = [note(1000, 0b00001, 'strum', 1500), note(1400, 0b00110), note(1600, 0b01000, 'hopo')];
    const engine = new Engine(track(notes), beats);
    press(engine, 'lane0', 990);
    press(engine, 'strum', 1000);
    press(engine, 'lane1', 1395);
    press(engine, 'lane2', 1396);
    press(engine, 'strum', 1400);
    press(engine, 'lane1', 1590, false);
    press(engine, 'lane2', 1590, false);
    press(engine, 'lane3', 1600);
    expect([...engine.judged]).toEqual([HIT, HIT, HIT]);
    expect(engine.isSustaining(0)).toBe(true);
    engine.update(2600);
    expect(engine.score).toBe(50 + 100 + 50 + 3 * 25);
  });

  it('drops a sustain on an overstrum or when its fret lets go', () => {
    const notes = [note(1000, 0b001, 'strum', 2000), note(2000, 0b010), note(4000, 0b001, 'strum', 1000)];
    const engine = new Engine(track(notes), beats);
    press(engine, 'lane0', 990);
    press(engine, 'strum', 1000);
    press(engine, 'strum', 1250);
    engine.update(1300);
    expect(engine.isSustaining(0)).toBe(false);
    expect(engine.overstrumAt).toBe(1275);
    expect(engine.score).toBe(50 + 13);
    press(engine, 'lane1', 1990);
    press(engine, 'strum', 2000);
    press(engine, 'lane1', 2100, false);
    press(engine, 'strum', 4000);
    expect(engine.isSustaining(2)).toBe(true);
    press(engine, 'lane0', 4100, false);
    engine.update(4200);
    expect(engine.isSustaining(2)).toBe(false);
  });

  it('forgives a brief let-go and an early release right at the end of a sustain', () => {
    const engine = new Engine(track([note(1000, 1, 'strum', 1000)]), beats);
    press(engine, 'lane0', 990);
    press(engine, 'strum', 1000);
    press(engine, 'lane0', 1500, false);
    press(engine, 'lane0', 1510);
    engine.update(1600);
    expect(engine.isSustaining(0)).toBe(true);
    press(engine, 'lane0', 1985, false);
    engine.update(2100);
    expect(engine.score).toBe(50 + 2 * 25);
  });

  it('waits a moment after a strum for the frets to catch up', () => {
    const engine = new Engine(track([note(1000, 1), note(2000, 2), note(3000, 4)]), beats);
    press(engine, 'strum', 1000);
    press(engine, 'lane0', 1030);
    expect(engine.judged[0]).toBe(HIT);
    press(engine, 'strum', 2000);
    press(engine, 'lane0', 2001, false);
    press(engine, 'lane1', 2060);
    expect(engine.judged[1]).toBe(PENDING);
    expect(engine.overstrumAt).toBe(2050);
    expect(engine.streak).toBe(0);
  });

  it('counts a quick second strum on nothing as an overstrum', () => {
    const engine = new Engine(track([note(1000, 1), note(3000, 1)]), beats);
    press(engine, 'lane0', 990);
    press(engine, 'strum', 1000);
    press(engine, 'strum', 1200);
    press(engine, 'strum', 1210);
    expect(engine.overstrumAt).toBe(1210);
  });

  it('eats one strum just after a hammer-on, but not a second', () => {
    const engine = new Engine(track([note(1000, 1), note(1100, 2, 'hopo'), note(3000, 2)]), beats);
    press(engine, 'lane0', 990);
    press(engine, 'strum', 1000);
    press(engine, 'lane1', 1100);
    press(engine, 'strum', 1175);
    engine.update(1250);
    expect(engine.streak).toBe(2);
    press(engine, 'strum', 1300);
    engine.update(1400);
    expect(engine.streak).toBe(0);
  });

  it('plays a hammer-on the moment its window opens when the fret is already held', () => {
    const engine = new Engine(track([note(1000, 1), note(1150, 2, 'hopo')]), beats);
    press(engine, 'lane0', 990);
    press(engine, 'strum', 1000);
    press(engine, 'lane1', 1010);
    expect(engine.judged[1]).toBe(PENDING);
    engine.update(1100);
    expect(engine.judged[1]).toBe(HIT);
    expect(engine.flashes[1]).toBe(1150 - EARLY_MS);
    expect(engine.streak).toBe(2);
  });

  it('keeps an early fret press for the next hammer-on after one played by fret', () => {
    const notes = [note(1000, 1), note(1100, 2, 'hopo'), note(1400, 4, 'hopo')];
    const engine = new Engine(track(notes), beats);
    press(engine, 'lane0', 990);
    press(engine, 'strum', 1000);
    press(engine, 'lane1', 1100);
    press(engine, 'lane2', 1200);
    engine.update(1500);
    expect([...engine.judged]).toEqual([HIT, HIT, HIT]);
  });

  it('judges held frets at the moment a window opened even across a long frame', () => {
    const engine = new Engine(track([note(1000, 1), note(1150, 2, 'hopo'), note(1250, 4, 'hopo')]), beats);
    press(engine, 'lane0', 990);
    press(engine, 'strum', 1000);
    press(engine, 'lane1', 1010);
    engine.update(1400);
    expect([...engine.judged]).toEqual([HIT, HIT, MISSED]);
    expect(engine.flashes[1]).toBe(1080);
  });

  it('plays a pull-off on the release that uncovers a held lower fret', () => {
    const notes = [note(1000, 0b011), note(1100, 0b001, 'hopo'), note(1200, 0b100, 'hopo'), note(1300, 0b010, 'hopo')];
    const engine = new Engine(track(notes), beats);
    press(engine, 'lane0', 990);
    press(engine, 'lane1', 990);
    press(engine, 'strum', 1000);
    press(engine, 'lane1', 1095, false);
    expect(engine.judged[1]).toBe(HIT);
    press(engine, 'lane1', 1150);
    press(engine, 'lane2', 1195);
    expect(engine.judged[2]).toBe(HIT);
    press(engine, 'lane2', 1290, false);
    expect(engine.judged[3]).toBe(HIT);
    expect(engine.streak).toBe(4);
  });

  it('plays anchored hammer-ons and a single hammer-on out of a held chord', () => {
    const notes = [note(1000, 0b00011), note(1100, 0b00010, 'hopo'), note(1300, 0b01000, 'hopo'), note(1400, 0b11000, 'hopo')];
    const engine = new Engine(track(notes), beats);
    press(engine, 'lane0', 990);
    press(engine, 'lane1', 990);
    press(engine, 'strum', 1000);
    engine.update(1100);
    expect(engine.judged[1]).toBe(HIT);
    press(engine, 'lane3', 1300);
    expect(engine.judged[2]).toBe(HIT);
    press(engine, 'lane4', 1400);
    expect([...engine.judged]).toEqual([HIT, HIT, HIT, HIT]);
  });

  it('needs one strum after a miss, then hammer-ons play by fret again', () => {
    const notes = [note(1000, 1), note(1300, 2, 'hopo'), note(1400, 4, 'hopo'), note(1500, 8, 'hopo')];
    const engine = new Engine(track(notes), beats);
    engine.update(1200);
    press(engine, 'lane1', 1300);
    expect(engine.judged[1]).toBe(PENDING);
    press(engine, 'strum', 1310);
    expect(engine.judged[1]).toBe(HIT);
    press(engine, 'lane2', 1400);
    press(engine, 'lane3', 1500);
    expect([...engine.judged]).toEqual([MISSED, HIT, HIT, HIT]);
  });

  it('logs timing from strums and fret presses but not from held frets', () => {
    const notes = [note(1000, 1), note(1100, 2, 'hopo'), note(1300, 4, 'hopo'), note(1600, 4)];
    const engine = new Engine(track(notes), beats);
    press(engine, 'lane0', 900);
    press(engine, 'strum', 1012);
    press(engine, 'lane1', 1095);
    press(engine, 'lane2', 1150);
    engine.update(1400);
    press(engine, 'strum', 1580);
    press(engine, 'lane2', 1580, false);
    press(engine, 'lane1', 1585, false);
    press(engine, 'lane2', 1590);
    expect([...engine.judged]).toEqual([HIT, HIT, HIT, HIT]);
    expect(engine.hitErrors).toEqual([12, -5, -20]);
  });

  it('skips past an unplayable note only once the streak is gone', () => {
    const notes = [note(1000, 1), note(1040, 2), note(1080, 4), note(2000, 1), note(2040, 2)];
    const engine = new Engine(track(notes), beats);
    press(engine, 'lane2', 990);
    press(engine, 'strum', 1050);
    expect([...engine.judged.slice(0, 3)]).toEqual([MISSED, MISSED, HIT]);
    expect(engine.streak).toBe(1);
    press(engine, 'lane2', 1900, false);
    press(engine, 'lane1', 1990);
    press(engine, 'strum', 2000);
    expect(engine.judged[4]).toBe(PENDING);
    engine.update(2060);
    expect(engine.streak).toBe(0);
  });

  it('breaks the star phrase being played on an overstrum', () => {
    const notes = [note(1000, 1), note(2000, 1), note(2500, 1)];
    const engine = new Engine(track(notes, 'guitar', [[1900, 2600]]), beats);
    press(engine, 'lane0', 990);
    press(engine, 'strum', 1000);
    press(engine, 'strum', 2000);
    press(engine, 'strum', 2200);
    press(engine, 'strum', 2500);
    engine.update(2700);
    expect(engine.inStarPhrase(2)).toBe(false);
    expect(engine.starMeter).toBe(0);
  });

  it('drains star power over eight measures', () => {
    const bars = Array.from({ length: 200 }, (_, i) => ({ t: i * 500, bar: i % 3 === 0 }));
    const notes = [note(1000, 1), note(1500, 1), note(2000, 1), note(2500, 1)];
    const engine = new Engine(track(notes, 'guitar', [[900, 1600], [1900, 2600]]), bars);
    press(engine, 'lane0', 0);
    for (const t of [1000, 1500, 2000, 2500]) press(engine, 'strum', t);
    press(engine, 'star', 3000);
    engine.update(3000 + 4 * 1500 - 10);
    expect(engine.starActive).toBe(true);
    engine.update(3000 + 4 * 1500 + 10);
    expect(engine.starActive).toBe(false);
  });
});
