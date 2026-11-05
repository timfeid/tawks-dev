import { describe, expect, it } from 'vitest';
import { AudioClock } from './audio';

describe('AudioClock', () => {
  /** A sound card 50 ppm fast whose output timestamps jitter by up to 5 ms, and land 48 ms behind the page. */
  const card = (perf: number, shift = 0) => {
    const truth = (perf * (1 + 50e-6) - 48 + shift) / 1000;
    return truth + (((perf * 7919) % 10) - 5) / 1000;
  };

  it('stays within a couple of milliseconds of the heard audio over a long song with no drift', () => {
    const clock = new AudioClock((t) => card(t));
    let worst = 0;
    for (let now = 0; now <= 6 * 60_000; now += 16) {
      const heard = clock.heardAt(now, now) * 1000;
      const truth = now * (1 + 50e-6) - 48;
      if (now > 2000) worst = Math.max(worst, Math.abs(heard - truth));
    }
    expect(worst).toBeLessThan(2.5);
  });

  it('follows a new output device at once', () => {
    let shift = 0;
    const clock = new AudioClock((t) => card(t, shift));
    for (let now = 0; now < 3000; now += 16) clock.heardAt(now, now);
    shift = -120;
    const heard = clock.heardAt(3000, 3000) * 1000;
    expect(Math.abs(heard - (3000 * (1 + 50e-6) - 168))).toBeLessThan(6);
  });
});
