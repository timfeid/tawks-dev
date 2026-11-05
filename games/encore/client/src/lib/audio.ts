import type { Instrument, SongInfo } from './types';

/** A part's stem drops this low while its player is missing, like the band losing them. */
const MISSING_GAIN = 0.06;
/** A stem fades out or back in over about 100 ms, so it never clicks. */
const FADE_TAU = 0.033;
// Each decoded stem is raw audio, tens of megabytes a minute, so only two at once.
const FETCH_AT_ONCE = 2;

let shared: AudioContext | null = null;

export function audioContext() {
  shared ??= new AudioContext({ latencyHint: 'interactive' });
  return shared;
}

/** Browsers hold audio until the page is used; call this from a click. */
export function unlockAudio() {
  void audioContext().resume();
}

/** Which stems carry each part. Bass charts usually play over `rhythm` when there is no bass stem. */
export function stemsFor(instrument: Instrument, song: SongInfo): string[] {
  const has = (stem: string) => song.stems.includes(stem);
  switch (instrument) {
    case 'bass':
      if (has('bass')) return ['bass'];
      return song.parts.some((p) => p.instrument === 'rhythm') ? [] : ['rhythm'];
    case 'drums':
      return ['drums', 'drums_1', 'drums_2', 'drums_3', 'drums_4'].filter(has);
    case 'vocals':
      return ['vocals', 'vocals_1'].filter(has);
    default:
      return has(instrument) ? [instrument] : [];
  }
}

type Group = { buffer: AudioBuffer; gain: GainNode; source: AudioBufferSourceNode | null };

/**
 * Plays a song as a backing mix plus one stem per part someone is playing,
 * all started on the same audio clock so muting a part never drifts.
 */
export class SongPlayer {
  readonly ctx = audioContext();
  private master: GainNode;
  private groups = new Map<Instrument | 'backing', Group>();
  /** Audio-clock seconds at which song time zero is heard. */
  private zero = 0;
  private playing = false;
  private silenced = new Set<Instrument>();

  constructor(volume = 0.9) {
    this.master = this.ctx.createGain();
    this.master.gain.value = volume;
    this.master.connect(this.ctx.destination);
  }

  async load(url: (stem: string) => string, song: SongInfo, parts: Instrument[], onProgress: (done: number, total: number) => void) {
    const owner = new Map<string, Instrument>();
    for (const part of parts) for (const stem of stemsFor(part, song)) owner.set(stem, part);

    const stems = song.stems.filter((s) => s !== 'preview' && s !== 'crowd');
    let done = 0;
    onProgress(0, stems.length);
    // Each stem is added into its group as soon as it decodes, so only the
    // mixes stay in memory rather than every stem at once.
    const mixes = new Map<Instrument | 'backing', AudioBuffer>();
    const queue = stems.slice();
    const worker = async () => {
      for (let stem = queue.shift(); stem; stem = queue.shift()) {
        const response = await fetch(url(stem));
        if (!response.ok) throw new Error(`The ${stem} track did not load`);
        const buffer = await this.ctx.decodeAudioData(await response.arrayBuffer());
        const key = owner.get(stem) ?? 'backing';
        mixes.set(key, mixInto(this.ctx, mixes.get(key), buffer));
        onProgress(++done, stems.length);
      }
    };
    await Promise.all(Array.from({ length: FETCH_AT_ONCE }, worker));

    for (const [key, buffer] of mixes) {
      const gain = this.ctx.createGain();
      gain.connect(this.master);
      this.groups.set(key, { buffer, gain, source: null });
    }
  }

  /** Starts so song time zero lands `inMs` from now; a negative value joins a song already playing. */
  start(inMs: number) {
    this.stop();
    const now = this.ctx.currentTime;
    this.zero = now + inMs / 1000;
    const when = Math.max(now + 0.05, this.zero);
    const offset = when - this.zero;
    for (const group of this.groups.values()) {
      const source = this.ctx.createBufferSource();
      source.buffer = group.buffer;
      source.connect(group.gain);
      source.start(when, offset);
      group.source = source;
    }
    this.playing = true;
  }

  stop() {
    for (const group of this.groups.values()) {
      try {
        group.source?.stop();
      } catch {
        // Already stopped.
      }
      group.source = null;
    }
    this.playing = false;
  }

  dispose() {
    this.stop();
    this.master.disconnect();
    this.groups.clear();
  }

  /** Drops a part's stem, or brings it back, when song time `atSongMs` is heard; right away without one. */
  setMissing(part: Instrument, missing: boolean, atSongMs?: number) {
    const group = this.groups.get(part);
    if (!group || this.silenced.has(part)) return;
    const now = this.ctx.currentTime;
    const at = atSongMs === undefined || !this.playing ? now : Math.max(now, this.contextTime(atSongMs));
    group.gain.gain.cancelScheduledValues(at);
    group.gain.gain.setTargetAtTime(missing ? MISSING_GAIN : 1, at, FADE_TAU);
  }

  /** Keeps a part's stem silent for this player, whatever the band does. */
  silence(part: Instrument) {
    this.silenced.add(part);
    const group = this.groups.get(part);
    group?.gain.gain.setTargetAtTime(0, this.ctx.currentTime, FADE_TAU);
  }

  /** A part's stem gain right now, for tests. */
  gainOf(part: Instrument | 'backing') {
    return this.groups.get(part)?.gain.gain.value ?? null;
  }

  /** The audio-clock time, in seconds, at which song time `songMs` plays. */
  contextTime(songMs: number) {
    return this.zero + songMs / 1000;
  }

  setVolume(volume: number) {
    this.master.gain.setTargetAtTime(volume, this.ctx.currentTime, 0.02);
  }

  /** The song time being heard at `performanceTime`, in milliseconds. */
  songTime(performanceTime = performance.now()) {
    if (!this.playing) return -Infinity;
    return (heardAt(this.ctx, performanceTime) - this.zero) * 1000;
  }
}

/** The audio-clock time that reaches the speakers at `performanceTime`. */
export function heardAt(ctx: AudioContext, performanceTime: number) {
  const stamp = ctx.getOutputTimestamp?.();
  if (stamp && stamp.contextTime && stamp.performanceTime) {
    return stamp.contextTime + (performanceTime - stamp.performanceTime) / 1000;
  }
  const latency = (ctx.outputLatency || 0) + (ctx.baseLatency || 0);
  return ctx.currentTime - latency + (performanceTime - performance.now()) / 1000;
}

/** Adds `buffer` into `into`, growing it when the new stem runs longer. */
function mixInto(ctx: AudioContext, into: AudioBuffer | undefined, buffer: AudioBuffer): AudioBuffer {
  if (!into) return buffer;
  let out = into;
  if (buffer.length > into.length || buffer.numberOfChannels > into.numberOfChannels) {
    out = ctx.createBuffer(Math.max(into.numberOfChannels, buffer.numberOfChannels), Math.max(into.length, buffer.length), ctx.sampleRate);
    for (let c = 0; c < out.numberOfChannels; c++) {
      out.getChannelData(c).set(into.getChannelData(Math.min(c, into.numberOfChannels - 1)));
    }
  }
  for (let c = 0; c < out.numberOfChannels; c++) {
    const target = out.getChannelData(c);
    const source = buffer.getChannelData(Math.min(c, buffer.numberOfChannels - 1));
    for (let i = 0; i < source.length; i++) target[i] += source[i];
  }
  return out;
}

/** A muted-string scratch for a strum with nothing to play. */
export function playScratch(volume = 0.5) {
  const ctx = audioContext();
  const now = ctx.currentTime;
  const length = Math.floor(ctx.sampleRate * 0.14);
  const buffer = ctx.createBuffer(1, length, ctx.sampleRate);
  const data = buffer.getChannelData(0);
  for (let i = 0; i < length; i++) data[i] = (Math.random() * 2 - 1) * (1 - i / length) ** 2;
  const noise = ctx.createBufferSource();
  noise.buffer = buffer;
  const band = ctx.createBiquadFilter();
  band.type = 'bandpass';
  band.frequency.setValueAtTime(2400, now);
  band.frequency.exponentialRampToValueAtTime(700, now + 0.12);
  band.Q.value = 1.2;
  const thump = ctx.createOscillator();
  thump.type = 'square';
  thump.frequency.setValueAtTime(130, now);
  thump.frequency.exponentialRampToValueAtTime(70, now + 0.08);
  const thumpGain = ctx.createGain();
  thumpGain.gain.setValueAtTime(volume * 0.25, now);
  thumpGain.gain.exponentialRampToValueAtTime(0.0001, now + 0.09);
  const gain = ctx.createGain();
  gain.gain.value = volume;
  noise.connect(band).connect(gain).connect(ctx.destination);
  thump.connect(thumpGain).connect(ctx.destination);
  noise.start(now);
  thump.start(now);
  thump.stop(now + 0.1);
}

/** Schedules metronome clicks and returns the audio-clock time of each. */
export function scheduleClicks(count: number, everyMs: number, leadMs = 600) {
  const ctx = audioContext();
  const first = ctx.currentTime + leadMs / 1000;
  const times: number[] = [];
  for (let i = 0; i < count; i++) {
    const at = first + (i * everyMs) / 1000;
    const osc = ctx.createOscillator();
    const gain = ctx.createGain();
    osc.frequency.value = i % 4 === 0 ? 1320 : 880;
    gain.gain.setValueAtTime(0.0001, at);
    gain.gain.exponentialRampToValueAtTime(0.5, at + 0.002);
    gain.gain.exponentialRampToValueAtTime(0.0001, at + 0.06);
    osc.connect(gain).connect(ctx.destination);
    osc.start(at);
    osc.stop(at + 0.08);
    times.push(at);
  }
  return times;
}
