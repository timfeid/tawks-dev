/** Small synthesized party sounds. Nothing plays until the page gets a tap or key press. */
export class PartySfx {
  private ctx: AudioContext | null = null;
  muted = false;

  private unlock = () => {
    if (!this.ctx) {
      try {
        this.ctx = new AudioContext();
      } catch {
        return;
      }
    }
    void this.ctx.resume();
  };

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
    try {
      localStorage.setItem('party-muted', muted ? '1' : '0');
    } catch {
      // Remembering the choice is a convenience only.
    }
  }

  private ready() {
    return !this.muted && this.ctx && this.ctx.state === 'running' ? this.ctx : null;
  }

  private tone(freqFrom: number, freqTo: number, duration: number, volume: number, type: OscillatorType, delay = 0) {
    const ctx = this.ready();
    if (!ctx) return;
    const start = ctx.currentTime + delay;
    const osc = ctx.createOscillator();
    const gain = ctx.createGain();
    osc.type = type;
    osc.frequency.setValueAtTime(freqFrom, start);
    osc.frequency.exponentialRampToValueAtTime(Math.max(20, freqTo), start + duration);
    gain.gain.setValueAtTime(0.0001, start);
    gain.gain.exponentialRampToValueAtTime(volume, start + 0.012);
    gain.gain.exponentialRampToValueAtTime(0.0001, start + duration);
    osc.connect(gain).connect(ctx.destination);
    osc.start(start);
    osc.stop(start + duration + 0.02);
  }

  bump(strength: number) {
    const s = Math.min(1, strength);
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
    source.connect(filter).connect(gain).connect(ctx.destination);
    source.start();
  }

  shot() {
    this.tone(760, 220, 0.09, 0.1, 'square');
    this.tone(180, 90, 0.12, 0.07, 'triangle');
  }

  splat() {
    this.tone(320, 120, 0.13, 0.14, 'sine');
  }

  down(mine: boolean) {
    this.tone(mine ? 300 : 660, mine ? 90 : 990, 0.3, 0.16, 'triangle');
  }

  /** A boot on concrete. Volume carries how close the walker is. */
  step(volume: number) {
    const ctx = this.ready();
    if (!ctx || volume <= 0.02) return;
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
    source.connect(filter).connect(gain).connect(ctx.destination);
    source.start();
  }

  /** The bomb's beep, tighter as the fuse runs down. */
  beep(urgency: number) {
    const pitch = 880 + urgency * 420;
    this.tone(pitch, pitch, 0.06, 0.08, 'square');
  }

  blast() {
    const ctx = this.ready();
    if (!ctx) return;
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
    gain.gain.value = 0.5;
    source.connect(filter).connect(gain).connect(ctx.destination);
    source.start();
    this.tone(120, 40, 0.8, 0.3, 'sine');
  }

  count(final: boolean) {
    this.tone(final ? 988 : 660, final ? 988 : 660, final ? 0.35 : 0.14, 0.16, 'square');
  }

  win() {
    [523, 659, 784, 1047].forEach((f, i) => this.tone(f, f, 0.22, 0.12, 'triangle', i * 0.11));
  }

  dispose() {
    window.removeEventListener('pointerdown', this.unlock);
    window.removeEventListener('keydown', this.unlock);
    void this.ctx?.close();
  }
}
