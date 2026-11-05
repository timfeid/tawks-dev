import { audioContext } from './audio';

/** What each drum lane sounds like: the kick, then red, yellow, blue and green. */
export const DRUM_VOICES = ['kick', 'snare', 'hihat', 'tom', 'crash'] as const;
export type DrumVoice = (typeof DRUM_VOICES)[number];

let noise: AudioBuffer | null = null;

function noiseBuffer(ctx: BaseAudioContext) {
  if (noise && noise.sampleRate === ctx.sampleRate) return noise;
  const length = ctx.sampleRate;
  noise = ctx.createBuffer(1, length, ctx.sampleRate);
  const data = noise.getChannelData(0);
  for (let i = 0; i < length; i++) data[i] = Math.random() * 2 - 1;
  return noise;
}

/** Synthesized drum hits, scheduled on the same audio clock as the song. */
export class DrumKit {
  private out: GainNode;

  constructor(
    private ctx: BaseAudioContext = audioContext(),
    volume = 0.8,
  ) {
    this.out = ctx.createGain();
    this.out.gain.value = volume;
    this.out.connect(ctx.destination);
  }

  setVolume(volume: number) {
    this.out.gain.value = volume;
  }

  get volume() {
    return this.out.gain.value;
  }

  /** Plays lane `lane`'s drum at audio-clock time `when`, or right away. */
  play(lane: number, when = this.ctx.currentTime) {
    if (this.out.gain.value <= 0) return;
    const at = Math.max(when, this.ctx.currentTime);
    switch (DRUM_VOICES[lane]) {
      case 'kick':
        this.tone(at, 'sine', 160, 45, 0.11, 1, 0.32);
        this.hiss(at, 'highpass', 3000, 0.25, 0.012);
        break;
      case 'snare':
        this.tone(at, 'triangle', 220, 160, 0.06, 0.5, 0.09);
        this.hiss(at, 'highpass', 1400, 0.75, 0.18);
        break;
      case 'hihat':
        this.hiss(at, 'highpass', 7500, 0.45, 0.06);
        break;
      case 'tom':
        this.tone(at, 'sine', 150, 95, 0.22, 0.85, 0.3);
        this.hiss(at, 'bandpass', 900, 0.12, 0.05);
        break;
      case 'crash':
        this.hiss(at, 'highpass', 4500, 0.5, 0.9);
        this.hiss(at, 'bandpass', 9000, 0.25, 0.5);
        break;
    }
  }

  dispose() {
    this.out.disconnect();
  }

  private tone(at: number, type: OscillatorType, from: number, to: number, sweep: number, level: number, decay: number) {
    const osc = this.ctx.createOscillator();
    osc.type = type;
    osc.frequency.setValueAtTime(from, at);
    osc.frequency.exponentialRampToValueAtTime(to, at + sweep);
    const gain = this.ctx.createGain();
    gain.gain.setValueAtTime(level, at);
    gain.gain.exponentialRampToValueAtTime(0.0001, at + decay);
    osc.connect(gain).connect(this.out);
    osc.start(at);
    osc.stop(at + decay + 0.02);
  }

  private hiss(at: number, type: BiquadFilterType, frequency: number, level: number, decay: number) {
    const source = this.ctx.createBufferSource();
    source.buffer = noiseBuffer(this.ctx);
    const filter = this.ctx.createBiquadFilter();
    filter.type = type;
    filter.frequency.value = frequency;
    const gain = this.ctx.createGain();
    gain.gain.setValueAtTime(level, at);
    gain.gain.exponentialRampToValueAtTime(0.0001, at + decay);
    source.connect(filter).connect(gain).connect(this.out);
    source.start(at);
    source.stop(at + decay + 0.02);
  }
}
