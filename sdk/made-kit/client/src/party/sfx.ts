import { watchSettings, type Settings } from './settings';

/** Kenney's CC0 audio packs, named `<sound>-<n>.ogg`; see sounds/README.md. */
const FILES = import.meta.glob('./sounds/*.ogg', { query: '?url', import: 'default', eager: true }) as Record<string, string>;
/** Each sound's files: a sound with several plays one of them at random. */
const BANK = new Map<string, string[]>();
for (const [path, url] of Object.entries(FILES)) {
  const name = path.replace(/^.*\//, '').replace(/-\d+\.ogg$/, '');
  BANK.set(name, [...(BANK.get(name) ?? []), url]);
}

/**
 * Party sounds: recorded ones from the sounds folder, with synthesized ones
 * for whatever has no recording, or until the recordings have loaded.
 * Nothing plays until the page gets a tap or key press.
 */
export class PartySfx {
  private ctx: AudioContext | null = null;
  private out: GainNode | null = null;
  private buffers = new Map<string, AudioBuffer>();
  private volume: Settings['sound'] = { master: 1, effects: 1, jingles: 1, ambience: 0.7 };
  private stopWatching = watchSettings((next) => {
    this.volume = next.sound;
    if (this.out) this.out.gain.value = this.muted || this.suspended ? 0 : next.sound.master * next.sound.effects;
  });
  muted = false;
  private suspended = false;

  private unlock = () => {
    if (!this.ctx) {
      try {
        this.ctx = new AudioContext();
      } catch {
        return;
      }
      this.out = this.ctx.createGain();
      this.out.gain.value = this.muted || this.suspended ? 0 : this.volume.master * this.volume.effects;
      this.out.connect(this.ctx.destination);
      void this.load(this.ctx);
    }
    void this.ctx.resume();
  };

  private async load(ctx: AudioContext) {
    await Promise.all(
      Object.values(FILES).map(async (url) => {
        try {
          const response = await fetch(url);
          this.buffers.set(url, await ctx.decodeAudioData(await response.arrayBuffer()));
        } catch {
          // A browser that cannot play Ogg keeps the synthesized sounds.
        }
      }),
    );
  }

  /** Plays one of a sound's recordings, a touch off pitch each time; false if it cannot. */
  private sample(name: string, volume: number, rate = 1, delay = 0) {
    const ctx = this.ready();
    const urls = (BANK.get(name) ?? []).filter((url) => this.buffers.has(url));
    if (!ctx || !this.out || !urls.length) return false;
    const source = ctx.createBufferSource();
    source.buffer = this.buffers.get(urls[Math.floor(Math.random() * urls.length)])!;
    source.playbackRate.value = rate * (0.95 + Math.random() * 0.1);
    const gain = ctx.createGain();
    gain.gain.value = name.startsWith('jingle') ? (volume * this.volume.jingles) / Math.max(0.05, this.volume.effects) : volume;
    source.connect(gain).connect(this.out);
    source.start(ctx.currentTime + delay);
    return true;
  }

  constructor() {
    try {
      this.muted = localStorage.getItem('party-muted') === '1';
    } catch {
      this.muted = false;
    }
    window.addEventListener('pointerdown', this.unlock);
    window.addEventListener('keydown', this.unlock);
  }

  setMuted(muted: boolean) {
    this.muted = muted;
    if (this.out) this.out.gain.value = muted || this.suspended ? 0 : this.volume.master * this.volume.effects;
    try {
      localStorage.setItem('party-muted', muted ? '1' : '0');
    } catch {
      // Remembering the choice is a convenience only.
    }
  }

  setSuspended(suspended: boolean) {
    this.suspended = suspended;
    if (this.out) this.out.gain.value = suspended || this.muted ? 0 : this.volume.master * this.volume.effects;
  }

  private ready() {
    return !this.muted && !this.suspended && this.ctx && this.ctx.state === 'running' ? this.ctx : null;
  }

  private tone(freqFrom: number, freqTo: number, duration: number, volume: number, type: OscillatorType, delay = 0, pan=0) {
    const ctx = this.ready();
    if (!ctx || volume<=.0001) return;
    const start = ctx.currentTime + delay;
    const osc = ctx.createOscillator();
    const gain = ctx.createGain();
    osc.type = type;
    osc.frequency.setValueAtTime(freqFrom, start);
    osc.frequency.exponentialRampToValueAtTime(Math.max(20, freqTo), start + duration);
    gain.gain.setValueAtTime(0.0001, start);
    gain.gain.exponentialRampToValueAtTime(volume, start + 0.012);
    gain.gain.exponentialRampToValueAtTime(0.0001, start + duration);
    const stereo=ctx.createStereoPanner();stereo.pan.value=pan;
    osc.connect(gain).connect(stereo).connect(this.out ?? ctx.destination);
    osc.start(start);
    osc.stop(start + duration + 0.02);
  }

  bump(strength: number) {
    const s = Math.min(1, strength);
    if (this.sample('punch', 0.3 + s * 0.5, 1.15 - s * 0.25)) return;
    this.tone(620 - s * 180, 150, 0.2 + s * 0.08, 0.12 + s * 0.2, 'sine');
    this.tone(1240 - s * 300, 400, 0.08, 0.04 + s * 0.05, 'triangle');
  }

  fall() {
    this.tone(700, 160, 0.55, 0.14, 'triangle');
  }

  splash() {
    const ctx = this.ready();
    if (!ctx) return;
    const length = Math.floor(ctx.sampleRate * 0.45);
    const buffer = ctx.createBuffer(1, length, ctx.sampleRate);
    const data = buffer.getChannelData(0);
    for (let i = 0; i < length; i++) data[i] = (Math.random() * 2 - 1) * (1 - i / length) ** 2;
    const source = ctx.createBufferSource();
    source.buffer = buffer;
    const filter = ctx.createBiquadFilter();
    filter.type = 'lowpass';
    filter.frequency.value = 1400;
    const gain = ctx.createGain();
    gain.gain.value = 0.35;
    source.connect(filter).connect(gain).connect(this.out ?? ctx.destination);
    source.start();
  }

  shot() {
    this.tone(760, 220, 0.09, 0.1, 'square');
    this.tone(180, 90, 0.12, 0.07, 'triangle');
  }

  splat() {
    if (this.sample('soft', 0.6)) return;
    this.tone(320, 120, 0.13, 0.14, 'sine');
  }

  down(mine: boolean) {
    this.sample('punch-heavy', mine ? 0.8 : 0.45);
    this.tone(mine ? 300 : 660, mine ? 90 : 990, 0.3, 0.1, 'triangle');
  }

  /** A boot on concrete. Volume carries how close the walker is. */
  step(volume: number) {
    const ctx = this.ready();
    if (!ctx || volume <= 0.02) return;
    if (this.sample('step-stone', 0.5 * Math.min(1, volume))) return;
    const length = Math.floor(ctx.sampleRate * 0.09);
    const buffer = ctx.createBuffer(1, length, ctx.sampleRate);
    const data = buffer.getChannelData(0);
    for (let i = 0; i < length; i++) data[i] = (Math.random() * 2 - 1) * (1 - i / length) ** 3;
    const source = ctx.createBufferSource();
    source.buffer = buffer;
    const filter = ctx.createBiquadFilter();
    filter.type = 'bandpass';
    filter.frequency.value = 430 + Math.random() * 120;
    filter.Q.value = 0.9;
    const gain = ctx.createGain();
    gain.gain.value = 0.22 * Math.min(1, volume);
    source.connect(filter).connect(gain).connect(this.out ?? ctx.destination);
    source.start();
  }

  /** The bomb's beep, tighter as the fuse runs down. */
  beep(urgency: number) {
    const pitch = 880 + urgency * 420;
    this.tone(pitch, pitch, 0.06, 0.08, 'square');
  }

  blast(volume = 1) {
    const ctx = this.ready();
    if (!ctx) return;
    this.sample('metal', 0.6 * volume, 0.7);
    const length = Math.floor(ctx.sampleRate * 1.1);
    const buffer = ctx.createBuffer(1, length, ctx.sampleRate);
    const data = buffer.getChannelData(0);
    for (let i = 0; i < length; i++) data[i] = (Math.random() * 2 - 1) * (1 - i / length) ** 1.6;
    const source = ctx.createBufferSource();
    source.buffer = buffer;
    const filter = ctx.createBiquadFilter();
    filter.type = 'lowpass';
    filter.frequency.setValueAtTime(900, ctx.currentTime);
    filter.frequency.exponentialRampToValueAtTime(120, ctx.currentTime + 0.9);
    const gain = ctx.createGain();
    gain.gain.value = 0.5 * volume;
    source.connect(filter).connect(gain).connect(this.out ?? ctx.destination);
    source.start();
    this.tone(120, 40, 0.8, 0.3 * volume, 'sine');
  }

  count(final: boolean) {
    this.tone(final ? 988 : 660, final ? 988 : 660, final ? 0.35 : 0.14, 0.16, 'square');
  }

  win() {
    if (this.sample('jingle-win', 0.6)) return;
    [523, 659, 784, 1047].forEach((f, i) => this.tone(f, f, 0.22, 0.12, 'triangle', i * 0.11));
  }

  coins(many = false) {
    this.sample(many ? 'coins' : 'chips', 0.55);
  }

  knockout() {
    this.hurt();
    this.sample('thud', 1, 0.7, 0.12);
    this.sample('lose', 0.6, 0.9, 0.2);
  }

  lose() {
    this.sample('lose', 0.45);
    this.sample('chips', 0.3, 0.75, 0.08);
  }

  item() {
    this.sample('power', 0.45);
  }

  land() {
    this.sample('thud', 0.8);
  }

  hurt() {
    if(!this.sample('punch-heavy', 0.7))this.tone(160,55,.12,.1,'triangle');
  }

  slide() {
    this.sample('ice', 0.6);
  }

  trap() {
    this.sample('metal', 0.6, 1.3);
  }

  blocked() {
    this.sample('blocked', 0.45);
  }

  prize() {
    this.sample('bell', 0.5);
    this.sample('jingle-prize', 0.6, 1, 0.25);
  }

  turn() {
    this.sample('confirm', 0.55, 1.1);
  }

  choose() {
    this.sample('question', 0.5);
  }

  taunt() {
    this.sample('pluck', 0.6);
  }

  round() {
    this.sample('jingle-round', 0.45);
  }

  over() {
    this.sample('jingle-over', 0.65);
  }

  dispose() {
    this.stopWatching();
    window.removeEventListener('pointerdown', this.unlock);
    window.removeEventListener('keydown', this.unlock);
    void this.ctx?.close();
  }
}
