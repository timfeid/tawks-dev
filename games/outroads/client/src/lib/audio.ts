/** A two-tone horn and a soft engine note, made on the fly so nothing has to be fetched. */

export class Sound {
  muted = false;
  private ctx: AudioContext | null = null;
  private engine: { osc: OscillatorNode; sub: OscillatorNode; gain: GainNode } | null = null;

  get context(): AudioContext | null {
    return this.ctx;
  }

  /** Browsers start audio only after a gesture; call this from one. */
  wake() {
    if (this.ctx) {
      void this.ctx.resume();
      return;
    }
    try {
      this.ctx = new AudioContext();
    } catch {
      return;
    }
    const ctx = this.ctx;
    const gain = ctx.createGain();
    gain.gain.value = 0;
    const filter = ctx.createBiquadFilter();
    filter.type = 'lowpass';
    filter.frequency.value = 520;
    const osc = ctx.createOscillator();
    osc.type = 'sawtooth';
    const sub = ctx.createOscillator();
    sub.type = 'triangle';
    osc.connect(filter);
    sub.connect(filter);
    filter.connect(gain).connect(ctx.destination);
    osc.start();
    sub.start();
    this.engine = { osc, sub, gain };
  }

  /** Engine pitch follows speed, with a little throttle bite. */
  setEngine(speed: number, throttle: number) {
    if (!this.ctx || !this.engine) return;
    const t = this.ctx.currentTime;
    const gear = Math.abs(speed) % 11;
    const rpm = 48 + gear * 9 + Math.abs(speed) * 1.4;
    this.engine.osc.frequency.setTargetAtTime(rpm, t, 0.08);
    this.engine.sub.frequency.setTargetAtTime(rpm / 2, t, 0.08);
    const level = this.muted ? 0 : 0.018 + throttle * 0.02;
    this.engine.gain.gain.setTargetAtTime(level, t, 0.1);
  }

  /** Volume 0 to 1; distant horns are quieter. */
  horn(volume = 1) {
    if (!this.ctx || this.muted || volume <= 0.01) return;
    const ctx = this.ctx;
    const t = ctx.currentTime;
    const gain = ctx.createGain();
    gain.gain.setValueAtTime(0, t);
    gain.gain.linearRampToValueAtTime(0.16 * volume, t + 0.02);
    gain.gain.setValueAtTime(0.16 * volume, t + 0.34);
    gain.gain.linearRampToValueAtTime(0, t + 0.42);
    const filter = ctx.createBiquadFilter();
    filter.type = 'bandpass';
    filter.frequency.value = 900;
    filter.Q.value = 0.8;
    filter.connect(gain).connect(ctx.destination);
    for (const f of [392, 494]) {
      const osc = ctx.createOscillator();
      osc.type = 'square';
      osc.frequency.value = f;
      osc.connect(filter);
      osc.start(t);
      osc.stop(t + 0.45);
    }
  }

  dispose() {
    void this.ctx?.close();
  }
}
