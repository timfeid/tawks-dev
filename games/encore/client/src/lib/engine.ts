import { OPEN, type Note, type Progress, type Track } from './types';

export type Action = 'lane0' | 'lane1' | 'lane2' | 'lane3' | 'lane4' | 'strum' | 'star';
/** `time` is song time in milliseconds, already corrected for latency. */
export type InputEvent = { action: Action; down: boolean; time: number };

export const EARLY_MS = 70;
export const LATE_MS = 70;
/** A strum this soon after a hammer-on or tap is the player strumming through it, not an extra strum. */
const STRUM_LENIENCY_MS = 60;
const NOTE_POINTS = 50;
const SUSTAIN_POINTS_PER_BEAT = 25;
const STAR_PHRASE_GAIN = 0.25;
const STAR_BEATS_PER_METER = 32;

export const PENDING = 0;
export const HIT = 1;
export const MISSED = 2;

function bits(lanes: number) {
  let count = 0;
  for (let n = lanes & 31; n; n &= n - 1) count++;
  return count;
}

function highest(lanes: number) {
  return lanes ? 1 << (31 - Math.clz32(lanes)) : 0;
}

/** Whether held frets play a note: single notes allow lower frets held as anchors, chords must be exact. */
export function fretsMatch(held: number, lanes: number) {
  const frets = lanes & 31;
  if (!frets) return held === 0;
  if (bits(frets) === 1) return highest(held) === frets;
  return held === frets;
}

type Phrase = { start: number; end: number; last: number; broken: boolean };

/** Judges one player's part against their input. Everything here runs on the player's own clock. */
export class Engine {
  readonly drums: boolean;
  readonly notes: Note[];
  readonly judged: Uint8Array;
  /** Last time each lane was hit, for the strike-line flash. */
  readonly flashes = [-1e9, -1e9, -1e9, -1e9, -1e9];
  /** Last time a note slipped past in each lane, for a red flash. */
  readonly missFlashes = [-1e9, -1e9, -1e9, -1e9, -1e9];
  /** Last strum or pad hit with nothing to play. */
  overstrumAt = -1e9;
  readonly end: number;
  held = 0;
  score = 0;
  streak = 0;
  bestStreak = 0;
  hit = 0;
  missed = 0;
  starMeter = 0;
  starActive = false;
  muted = false;
  /** The sustain being held, by note index, with how far it has been scored. */
  sustain: { index: number; end: number; lanes: number; scoredTo: number } | null = null;
  lastMissAt = -1e9;
  private next = 0;
  private lastTime = 0;
  private lastLenientHit = -1e9;
  private sustainCarry = 0;
  private phrases: Phrase[];
  private phraseOf: Int32Array;

  constructor(
    track: Track,
    private beats: { t: number }[],
  ) {
    this.drums = track.instrument === 'drums';
    this.notes = this.drums ? splitGems(track.notes) : track.notes.slice();
    this.judged = new Uint8Array(this.notes.length);
    const last = this.notes[this.notes.length - 1];
    this.end = last ? last.t + last.len + LATE_MS : 0;
    this.phraseOf = new Int32Array(this.notes.length).fill(-1);
    this.phrases = [];
    for (const [start, end] of track.star_power) {
      const phrase: Phrase = { start, end, last: -1, broken: false };
      this.notes.forEach((note, i) => {
        if (note.t >= start && note.t <= end) {
          this.phraseOf[i] = this.phrases.length;
          phrase.last = i;
        }
      });
      if (phrase.last >= 0) this.phrases.push(phrase);
    }
  }

  get multiplier() {
    const base = Math.min(4, 1 + Math.floor(this.streak / 10));
    return this.starActive ? base * 2 : base;
  }

  inStarPhrase(index: number) {
    const phrase = this.phraseOf[index];
    return phrase >= 0 && !this.phrases[phrase].broken;
  }

  input(event: InputEvent) {
    this.update(event.time);
    const { action, down, time } = event;
    if (action === 'star') {
      if (down && !this.starActive && this.starMeter >= 0.5) this.starActive = true;
      return;
    }
    if (this.drums) {
      if (down && action !== 'strum') this.pad(Number(action.slice(4)), time);
      return;
    }
    if (action === 'strum') {
      if (down) this.strum(time);
      return;
    }
    const bit = 1 << Number(action.slice(4));
    const before = this.held;
    this.held = down ? this.held | bit : this.held & ~bit;
    if (this.held !== before) this.fretChange(time);
  }

  /** Marks notes that slipped past, scores sustains and drains star power up to `time`. */
  update(time: number) {
    if (time < this.lastTime) return;
    const dt = time - this.lastTime;
    this.lastTime = time;

    while (this.next < this.notes.length && this.judged[this.next] !== PENDING) this.next++;
    for (let i = this.next; i < this.notes.length && this.notes[i].t + LATE_MS < time; i++) {
      if (this.judged[i] === PENDING) this.miss(i, time);
    }

    if (this.sustain) {
      const frets = this.sustain.lanes & 31;
      const held = frets ? (this.held & frets) === frets : this.held === 0;
      const to = Math.min(time, this.sustain.end);
      if (held) {
        this.sustainCarry += ((to - this.sustain.scoredTo) / this.beatLength(to)) * SUSTAIN_POINTS_PER_BEAT * this.multiplier;
        const whole = Math.floor(this.sustainCarry);
        this.score += whole;
        this.sustainCarry -= whole;
        this.sustain.scoredTo = to;
      }
      if (!held || time >= this.sustain.end) {
        this.sustain = null;
        this.sustainCarry = 0;
      }
    }

    if (this.starActive) {
      this.starMeter -= dt / (STAR_BEATS_PER_METER * this.beatLength(time));
      if (this.starMeter <= 0) {
        this.starMeter = 0;
        this.starActive = false;
      }
    }
  }

  get done() {
    return this.lastTime > this.end;
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
      star_meter: Math.round(this.starMeter * 100) / 100,
      star_active: this.starActive,
      muted: this.muted,
    };
  }

  private beatLength(time: number) {
    const beats = this.beats;
    let lo = 0;
    let hi = beats.length - 1;
    while (lo < hi) {
      const mid = (lo + hi + 1) >> 1;
      if (beats[mid].t <= time) lo = mid;
      else hi = mid - 1;
    }
    const a = beats[lo];
    const b = beats[lo + 1] ?? beats[lo - 1];
    return a && b ? Math.max(100, Math.abs(b.t - a.t)) : 500;
  }

  /** The earliest unjudged note whose window contains `time`. */
  private candidate(time: number) {
    for (let i = this.next; i < this.notes.length; i++) {
      const note = this.notes[i];
      if (note.t - EARLY_MS > time) return -1;
      if (this.judged[i] === PENDING && note.t + LATE_MS >= time) return i;
    }
    return -1;
  }

  private strum(time: number) {
    if (time - this.lastLenientHit < STRUM_LENIENCY_MS) {
      this.lastLenientHit = -1e9;
      return;
    }
    const i = this.candidate(time);
    if (i >= 0 && fretsMatch(this.held, this.notes[i].lanes)) this.hitNote(i, time);
    else {
      this.overstrumAt = time;
      this.breakStreak(time);
    }
  }

  private fretChange(time: number) {
    const i = this.candidate(time);
    if (i < 0) return;
    const note = this.notes[i];
    const lenient = note.kind === 'tap' || (note.kind === 'hopo' && this.streak > 0);
    if (lenient && fretsMatch(this.held, note.lanes)) {
      this.hitNote(i, time);
      this.lastLenientHit = time;
    }
  }

  private pad(lane: number, time: number) {
    const bit = 1 << lane;
    let best = -1;
    for (let i = this.next; i < this.notes.length; i++) {
      const note = this.notes[i];
      if (note.t - EARLY_MS > time) break;
      if (this.judged[i] !== PENDING || !(note.lanes & bit) || note.t + LATE_MS < time) continue;
      best = i;
      break;
    }
    if (best >= 0) this.hitNote(best, time);
  }

  private hitNote(i: number, time: number) {
    const note = this.notes[i];
    this.judged[i] = HIT;
    this.hit++;
    this.score += NOTE_POINTS * Math.max(1, bits(note.lanes)) * this.multiplier;
    this.streak++;
    this.bestStreak = Math.max(this.bestStreak, this.streak);
    this.muted = false;
    for (let lane = 0; lane < 5; lane++) if (note.lanes & (1 << lane)) this.flashes[lane] = time;
    if (note.lanes & OPEN) this.flashes.fill(time);
    if (!this.drums && note.len > 0) {
      this.sustain = { index: i, end: note.t + note.len, lanes: note.lanes, scoredTo: Math.max(time, note.t) };
      this.sustainCarry = 0;
    }
    const phrase = this.phraseOf[i];
    if (phrase >= 0 && this.phrases[phrase].last === i && !this.phrases[phrase].broken) {
      this.starMeter = Math.min(1, this.starMeter + STAR_PHRASE_GAIN);
    }
  }

  private miss(i: number, time: number) {
    this.judged[i] = MISSED;
    this.missed++;
    const lanes = this.notes[i].lanes;
    for (let lane = 0; lane < 5; lane++) if (lanes & (1 << lane)) this.missFlashes[lane] = time;
    if (lanes & OPEN) this.missFlashes.fill(time);
    const phrase = this.phraseOf[i];
    if (phrase >= 0) this.phrases[phrase].broken = true;
    this.breakStreak(time);
  }

  private breakStreak(time: number) {
    this.streak = 0;
    this.muted = true;
    this.lastMissAt = time;
  }
}

function splitGems(notes: Note[]) {
  const gems: Note[] = [];
  for (const note of notes) {
    for (let lane = 0; lane < 5; lane++) {
      if (note.lanes & (1 << lane)) gems.push({ t: note.t, len: 0, lanes: 1 << lane, kind: 'strum' });
    }
  }
  return gems;
}
