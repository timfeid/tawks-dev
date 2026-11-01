import { HIT, MISSED, PENDING } from './engine';
import type { Difficulty, Progress, VocalNote } from './types';

/** How far off, in semitones, still counts as on pitch. */
const TOLERANCE: Record<Difficulty, number> = { easy: 3.5, medium: 2.5, hard: 1.5, expert: 1 };
/** A note counts as sung when the voice was on it for this much of its length. */
const HIT_SHARE = 0.5;
const MIN_NOTE_MS = 120;
/** How long after a note ends before it is judged, so late samples still count. */
const JUDGE_AFTER_MS = 150;
const SPOKEN_LEVEL = 0.02;
const NOTE_POINTS = 50;
const TRAIL_MS = 2_000;

/** One reading of the singer, on the song's clock. */
export type VoiceSample = { time: number; midi: number | null; level: number };

export function hzToMidi(hz: number) {
  return 69 + 12 * Math.log2(hz / 440);
}

/** Distance in semitones ignoring octaves, so singing an octave off still counts. */
export function pitchDistance(midi: number, target: number) {
  const d = (((midi - target) % 12) + 18) % 12 - 6;
  return Math.abs(d);
}

/** Judges a singer's pitch against the vocal chart, note by note, on their own clock. */
export class VocalEngine {
  readonly notes: VocalNote[];
  readonly judged: Uint8Array;
  /** Milliseconds of each note sung on pitch. */
  readonly credit: Float64Array;
  /** Recent voice for drawing; `shown` is the pitch moved to the octave nearest the note. */
  readonly trail: (VoiceSample & { shown: number | null })[] = [];
  readonly end: number;
  readonly tolerance: number;
  score = 0;
  streak = 0;
  bestStreak = 0;
  hit = 0;
  missed = 0;
  muted = false;
  lastMissAt = -1e9;
  readonly starMeter = 0;
  readonly starActive = false;
  private next = 0;
  private lastSample = -1e9;
  private lastTime = 0;
  private lastTarget: number | null = null;

  constructor(notes: VocalNote[], difficulty: Difficulty) {
    this.notes = notes.filter((n) => n.len > 0);
    this.judged = new Uint8Array(this.notes.length);
    this.credit = new Float64Array(this.notes.length);
    this.tolerance = TOLERANCE[difficulty];
    const last = this.notes[this.notes.length - 1];
    this.end = last ? last.t + last.len + JUDGE_AFTER_MS : 0;
  }

  get multiplier() {
    return Math.min(4, 1 + Math.floor(this.streak / 10));
  }

  get done() {
    return this.lastTime > this.end;
  }

  /** The note being sung at `time`, if any. */
  noteAt(time: number) {
    for (let i = this.next; i < this.notes.length; i++) {
      const note = this.notes[i];
      if (note.t > time) return -1;
      if (time <= note.t + Math.max(note.len, MIN_NOTE_MS)) return i;
    }
    return -1;
  }

  feed(samples: VoiceSample[]) {
    for (const sample of samples) {
      const dt = Math.min(40, Math.max(0, sample.time - this.lastSample));
      this.lastSample = sample.time;
      const i = this.noteAt(sample.time);
      if (i >= 0 && !this.notes[i].talky) this.lastTarget = this.notes[i].pitch;
      const around = this.lastTarget ?? sample.midi;
      const shown =
        sample.midi === null || around === null ? null : sample.midi + 12 * Math.round((around - sample.midi) / 12);
      this.trail.push({ ...sample, shown });
      if (i < 0 || this.judged[i] !== PENDING) continue;
      const note = this.notes[i];
      const sung = note.talky
        ? sample.level >= SPOKEN_LEVEL
        : sample.midi !== null && pitchDistance(sample.midi, note.pitch) <= this.tolerance;
      if (sung) this.credit[i] += dt;
    }
    while (this.trail.length && this.trail[0].time < this.lastSample - TRAIL_MS) this.trail.shift();
  }

  update(time: number) {
    if (time < this.lastTime) return;
    this.lastTime = time;
    while (this.next < this.notes.length) {
      const note = this.notes[this.next];
      if (note.t + note.len + JUDGE_AFTER_MS > time) break;
      const share = this.credit[this.next] / Math.max(note.len, MIN_NOTE_MS);
      if (share >= HIT_SHARE) {
        this.judged[this.next] = HIT;
        this.hit++;
        this.score += Math.round(NOTE_POINTS * Math.min(1, share) * this.multiplier);
        this.streak++;
        this.bestStreak = Math.max(this.bestStreak, this.streak);
        this.muted = false;
      } else {
        this.judged[this.next] = MISSED;
        this.missed++;
        this.streak = 0;
        this.muted = true;
        this.lastMissAt = time;
      }
      this.next++;
    }
  }

  progress(): Progress {
    return {
      score: this.score,
      streak: this.streak,
      best_streak: this.bestStreak,
      multiplier: this.multiplier,
      hit: this.hit,
      missed: this.missed,
      total: this.notes.length,
      star_meter: 0,
      star_active: false,
      muted: this.muted,
    };
  }
}
