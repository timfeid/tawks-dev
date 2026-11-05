import { OPEN, type Note, type Progress, type Track } from './types';

export type Action = 'lane0' | 'lane1' | 'lane2' | 'lane3' | 'lane4' | 'strum' | 'star';
/** `time` is song time in milliseconds, already corrected for latency. */
export type InputEvent = { action: Action; down: boolean; time: number };

export const EARLY_MS = 70;
export const LATE_MS = 70;
/** A strum waits this long for the frets to catch up before it counts as an overstrum. */
export const STRUM_LENIENCY_MS = 50;
/** The wait when no note was in the window at the strum. */
export const STRUM_LENIENCY_SMALL_MS = 25;
/** A strum this soon after a hammer-on or tap is the player strumming through it, not an extra strum. */
export const HOPO_LENIENCY_MS = 80;
/** Letting go of a sustain this briefly, or this close to its end, does not drop it. */
export const SUSTAIN_DROP_LENIENCY_MS = 25;
const NOTE_POINTS = 50;
const SUSTAIN_POINTS_PER_BEAT = 25;
const STAR_PHRASE_GAIN = 0.25;
const STAR_MEASURES_PER_METER = 8;

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

function lowest(lanes: number) {
  return lanes & -lanes;
}

/**
 * Whether held frets play a note. Single notes allow lower frets held as
 * anchors; strummed chords must be exact, while hammer-on and tap chords may
 * be anchored below their lowest fret. Open chords never anchor.
 */
export function fretsMatch(held: number, lanes: number, kind: Note['kind'] = 'strum') {
  const frets = lanes & 31;
  if (lanes & OPEN) return held === frets;
  if (bits(frets) === 1) return highest(held) === frets;
  if (held === frets) return true;
  if (kind === 'strum' || (held & frets) !== frets) return false;
  const anchors = held & ~frets;
  return highest(anchors) < lowest(frets);
}

type Phrase = { start: number; end: number; first: number; last: number; broken: boolean };

/** A sustain being held, by note index, with how far it has been scored. */
export type Sustain = {
  index: number;
  end: number;
  lanes: number;
  scoredTo: number;
  /** It runs past the start of the next note, so other frets may be played over it. */
  extended: boolean;
  releasedAt: number | null;
};

/**
 * Judges one player's part against their input, following Clone Hero's
 * five-fret rules. Everything here runs on the player's own clock.
 */
export class Engine {
  readonly drums: boolean;
  readonly notes: Note[];
  readonly judged: Uint8Array;
  /** Last time each lane was hit, for the strike-line flash. */
  readonly flashes = [-1e9, -1e9, -1e9, -1e9, -1e9];
  /** Last time a note slipped past in each lane, for a red flash. */
  readonly missFlashes = [-1e9, -1e9, -1e9, -1e9, -1e9];
  /** Last time each lane was pressed, note or not, so every press lights its target. */
  readonly presses = [-1e9, -1e9, -1e9, -1e9, -1e9];
  /** Last strum with nothing to play. */
  overstrumAt = -1e9;
  /** Kick notes play themselves; kick presses are ignored. */
  readonly autoKick: boolean;
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
  /** Song time `muted` last changed, for the band to drop or restore this part's stem at. */
  mutedAt = -1e9;
  readonly sustains: Sustain[] = [];
  lastMissAt = -1e9;
  private next = 0;
  private lastTime = 0;
  private sustainCarry = 0;
  /** When the strum still waiting for frets runs out, or null. */
  private strumUntil: number | null = null;
  private hopoLeniencyUntil = -1e9;
  /** A fret change (or a strummed hit) that hammer-ons and taps may still be played with. */
  private tapped = true;
  /** Hammer-ons and taps starting after this need a newer fret change. */
  private tapFrontEnd = Infinity;
  /** A wrong fret was hammered during the note's window, so it needs a strum. */
  private ghosted = false;
  private phrases: Phrase[];
  private phraseOf: Int32Array;
  private bars: number[];

  constructor(
    track: Track,
    private beats: { t: number; bar?: boolean }[],
    options: { autoKick?: boolean } = {},
  ) {
    this.drums = track.instrument === 'drums';
    this.autoKick = this.drums && !!options.autoKick;
    this.notes = this.drums ? splitGems(track.notes) : track.notes.slice();
    this.judged = new Uint8Array(this.notes.length);
    const last = this.notes[this.notes.length - 1];
    this.end = last ? last.t + last.len + LATE_MS : 0;
    this.bars = beats.filter((b) => b.bar).map((b) => b.t);
    this.phraseOf = new Int32Array(this.notes.length).fill(-1);
    this.phrases = [];
    for (const [start, end] of track.star_power) {
      const phrase: Phrase = { start, end, first: -1, last: -1, broken: false };
      this.notes.forEach((note, i) => {
        if (note.t >= start && note.t <= end) {
          this.phraseOf[i] = this.phrases.length;
          if (phrase.first < 0) phrase.first = i;
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

  isSustaining(index: number) {
    return this.sustains.some((s) => s.index === index);
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
      if (!down || action === 'strum') return;
      const lane = Number(action.slice(4));
      if (lane === 0 && this.autoKick) return;
      this.presses[lane] = time;
      this.pad(lane, time);
      return;
    }
    if (action === 'strum') {
      if (down) this.strum(time);
      return;
    }
    const lane = Number(action.slice(4));
    const bit = 1 << lane;
    if (down) this.presses[lane] = time;
    const before = this.held;
    this.held = down ? this.held | bit : this.held & ~bit;
    if (this.held !== before) this.fretChange(before, time);
  }

  /** Marks notes that slipped past, settles waiting strums, scores sustains and drains star power up to `time`. */
  update(time: number) {
    if (time < this.lastTime) return;
    const dt = time - this.lastTime;
    this.lastTime = time;

    if (this.autoKick) this.kickUntil(time);
    if (this.strumUntil !== null && this.strumUntil < time) {
      const until = this.strumUntil;
      this.missUntil(until);
      this.tryHit(until);
      if (this.strumUntil !== null) {
        this.strumUntil = null;
        this.overstrum(until);
      }
    }
    this.missUntil(time);
    if (!this.drums && (this.strumUntil !== null || this.tapped)) this.tryHit(time);
    this.holdSustains(time);

    if (this.starActive) {
      this.starMeter -= dt / (STAR_MEASURES_PER_METER * this.measureLength(time));
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
      auto_kick: this.autoKick,
    };
  }

  /** The kick notes that play themselves, for drawing and scheduling their sound. */
  isAutoKick(index: number) {
    return this.autoKick && this.notes[index].lanes === 1;
  }

  private kickUntil(time: number) {
    for (let i = this.next; i < this.notes.length && this.notes[i].t <= time; i++) {
      if (this.judged[i] === PENDING && this.notes[i].lanes === 1) {
        this.presses[0] = this.notes[i].t;
        this.hitNote(i, this.notes[i].t);
      }
    }
  }

  private missUntil(time: number) {
    while (this.next < this.notes.length && this.judged[this.next] !== PENDING) this.next++;
    for (let i = this.next; i < this.notes.length && this.notes[i].t + LATE_MS < time; i++) {
      if (this.judged[i] === PENDING) this.miss(i, time);
    }
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

  private measureLength(time: number) {
    const bars = this.bars;
    if (bars.length < 2) return 4 * this.beatLength(time);
    let lo = 0;
    let hi = bars.length - 1;
    while (lo < hi) {
      const mid = (lo + hi + 1) >> 1;
      if (bars[mid] <= time) lo = mid;
      else hi = mid - 1;
    }
    const a = bars[lo];
    const b = bars[lo + 1] ?? bars[lo - 1];
    return Math.max(400, Math.abs(b - a));
  }

  private inWindow(note: Note, time: number) {
    return note.t - EARLY_MS <= time && time <= note.t + LATE_MS;
  }

  private judgedAny() {
    return this.hit + this.missed > 0;
  }

  /** Frets of extended sustains being held, which other notes are judged without. */
  private extendedFrets() {
    let frets = 0;
    for (const s of this.sustains) if (s.extended) frets |= s.lanes & 31;
    return frets;
  }

  private canHit(note: Note) {
    const extended = this.extendedFrets();
    const masked = this.held & ~extended;
    if (masked !== this.held && fretsMatch(masked, note.lanes, note.kind)) return true;
    return fretsMatch(this.held, note.lanes, note.kind);
  }

  /** Plays the first note the current frets, strum and fret changes allow, missing any skipped. */
  private tryHit(time: number) {
    let first = true;
    for (let i = this.next; i < this.notes.length; i++) {
      if (this.judged[i] !== PENDING) continue;
      const note = this.notes[i];
      if (!this.inWindow(note, time)) break;
      const isFirst = first;
      first = false;
      if (!this.canHit(note)) {
        if ((note.kind === 'hopo' || note.kind === 'tap') && !this.judgedAny()) break;
        continue;
      }
      const hopo = note.kind === 'hopo' && isFirst && this.streak > 0;
      const tap = note.kind === 'tap' && (isFirst || this.streak === 0);
      if (this.tapped && (hopo || tap) && note.t <= this.tapFrontEnd && !this.ghosted) {
        this.hitNote(i, time);
        return true;
      }
      if (this.strumUntil !== null && (isFirst || this.streak === 0)) {
        this.hitNote(i, time);
        return true;
      }
    }
    return false;
  }

  private strum(time: number) {
    if (time <= this.hopoLeniencyUntil) {
      this.hopoLeniencyUntil = -1e9;
      this.strumUntil = null;
      return;
    }
    if (this.strumUntil !== null) this.overstrum(time);
    const first = this.notes[this.next];
    const near = !!first && this.inWindow(first, time);
    this.strumUntil = time + (near ? STRUM_LENIENCY_MS : STRUM_LENIENCY_SMALL_MS);
    this.tryHit(time);
  }

  private fretChange(before: number, time: number) {
    this.tapped = true;
    this.tapFrontEnd = time + EARLY_MS;
    const first = this.notes[this.next];
    const hammered = highest(this.held) > highest(before);
    if (first && this.next > 0 && hammered && this.inWindow(first, time) && !(this.held & first.lanes & 31)) {
      this.ghosted = true;
    }
    this.tryHit(time);
    this.holdSustains(time);
  }

  private holding(s: Sustain) {
    const frets = s.lanes & 31;
    if (s.extended) return s.lanes & OPEN ? true : (this.held & frets) === frets;
    let others = 0;
    for (const o of this.sustains) if (o !== s && o.extended) others |= o.lanes & 31;
    const note = this.notes[s.index];
    return fretsMatch(this.held & ~others, s.lanes, note.kind) || fretsMatch(this.held, s.lanes, note.kind);
  }

  private holdSustains(time: number) {
    for (let k = 0; k < this.sustains.length; k++) {
      const s = this.sustains[k];
      if (this.holding(s)) {
        s.releasedAt = null;
        this.scoreSustain(s, Math.min(time, s.end));
      } else {
        s.releasedAt ??= time;
      }
      const dropped = s.releasedAt !== null && time - s.releasedAt > SUSTAIN_DROP_LENIENCY_MS;
      if (time >= s.end && (s.releasedAt === null || s.end - s.releasedAt <= SUSTAIN_DROP_LENIENCY_MS)) {
        this.scoreSustain(s, s.end);
        this.sustains.splice(k--, 1);
      } else if (dropped) {
        this.sustains.splice(k--, 1);
      }
    }
    if (!this.sustains.length) this.sustainCarry = 0;
  }

  private scoreSustain(s: Sustain, to: number) {
    if (to <= s.scoredTo) return;
    this.sustainCarry += ((to - s.scoredTo) / this.beatLength(to)) * SUSTAIN_POINTS_PER_BEAT * this.multiplier;
    const whole = Math.floor(this.sustainCarry);
    this.score += whole;
    this.sustainCarry -= whole;
    s.scoredTo = to;
  }

  private endSustains(time: number, lanes = ~0) {
    for (let k = 0; k < this.sustains.length; k++) {
      const s = this.sustains[k];
      if (!(s.lanes & lanes)) continue;
      if (s.releasedAt === null) this.scoreSustain(s, Math.min(time, s.end));
      this.sustains.splice(k--, 1);
    }
  }

  private overstrum(time: number) {
    if (!this.judgedAny()) return;
    if (this.next >= this.notes.length && !this.sustains.length) return;
    this.overstrumAt = time;
    this.endSustains(time);
    let upcoming = this.next;
    while (upcoming < this.notes.length && this.judged[upcoming] !== PENDING) upcoming++;
    const phrase = upcoming < this.notes.length ? this.phraseOf[upcoming] : -1;
    if (phrase >= 0 && this.phrases[phrase].first !== upcoming) this.phrases[phrase].broken = true;
    this.breakStreak(time);
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
    if (!this.drums) {
      for (let k = this.next; k < i; k++) if (this.judged[k] === PENDING) this.miss(k, time);
      if (note.kind === 'hopo' || note.kind === 'tap') {
        this.tapped = false;
        this.hopoLeniencyUntil = time + HOPO_LENIENCY_MS;
      } else {
        this.tapped = true;
        this.tapFrontEnd = Infinity;
      }
      this.strumUntil = null;
      this.ghosted = false;
      this.endSustains(time, note.lanes & OPEN ? OPEN : note.lanes & 31);
    }
    this.judged[i] = HIT;
    this.hit++;
    this.score += NOTE_POINTS * Math.max(1, bits(note.lanes)) * this.multiplier;
    this.streak++;
    this.bestStreak = Math.max(this.bestStreak, this.streak);
    this.setMuted(false, time);
    for (let lane = 0; lane < 5; lane++) if (note.lanes & (1 << lane)) this.flashes[lane] = time;
    if (note.lanes & OPEN) this.flashes.fill(time);
    if (!this.drums && note.len > 0) {
      const end = note.t + note.len;
      const after = this.notes[i + 1];
      this.sustains.push({
        index: i,
        end,
        lanes: note.lanes,
        scoredTo: Math.max(time, note.t),
        extended: !!after && after.t < end,
        releasedAt: null,
      });
    }
    const phrase = this.phraseOf[i];
    if (phrase >= 0 && this.phrases[phrase].last === i && !this.phrases[phrase].broken) {
      this.starMeter = Math.min(1, this.starMeter + STAR_PHRASE_GAIN);
    }
  }

  private miss(i: number, time: number) {
    this.judged[i] = MISSED;
    this.missed++;
    this.tapped = false;
    this.ghosted = false;
    const lanes = this.notes[i].lanes;
    for (let lane = 0; lane < 5; lane++) if (lanes & (1 << lane)) this.missFlashes[lane] = time;
    if (lanes & OPEN) this.missFlashes.fill(time);
    const phrase = this.phraseOf[i];
    if (phrase >= 0) this.phrases[phrase].broken = true;
    this.breakStreak(time);
  }

  private breakStreak(time: number) {
    this.streak = 0;
    this.setMuted(true, time);
    this.lastMissAt = time;
  }

  private setMuted(muted: boolean, time: number) {
    if (this.muted === muted) return;
    this.muted = muted;
    this.mutedAt = time;
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
