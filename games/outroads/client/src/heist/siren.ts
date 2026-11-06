/**
 * Sirens and alarms made on the fly. Each siren rises and falls, pans and
 * fades with distance, and bends in pitch with how fast it closes on the
 * listener, since the Web Audio panner no longer does Doppler.
 */

const SOUND_SPEED = 343;

type Voice = { osc: OscillatorNode; gain: GainNode; pan: PannerNode; phase: number; last: { x: number; z: number } | null; yelp: boolean };

export class Sirens {
  private ctx: AudioContext | null = null;
  private voices = new Map<string, Voice>();
  private out: GainNode | null = null;
  muted = false;

  wake(ctx: AudioContext | null) {
    if (this.ctx || !ctx) return;
    this.ctx = ctx;
    this.out = ctx.createGain();
    this.out.gain.value = 0.5;
    this.out.connect(ctx.destination);
  }

  private voice(id: string): Voice | null {
    const ctx = this.ctx;
    if (!ctx || !this.out) return null;
    let v = this.voices.get(id);
    if (v) return v;
    const osc = ctx.createOscillator();
    osc.type = 'sawtooth';
    const filter = ctx.createBiquadFilter();
    filter.type = 'lowpass';
    filter.frequency.value = 2400;
    const gain = ctx.createGain();
    gain.gain.value = 0;
    const pan = ctx.createPanner();
    pan.panningModel = 'equalpower';
    pan.distanceModel = 'inverse';
    pan.refDistance = 18;
    pan.rolloffFactor = 1.1;
    pan.maxDistance = 900;
    osc.connect(filter).connect(gain).connect(pan).connect(this.out);
    osc.start();
    v = { osc, gain, pan, phase: Math.random(), last: null, yelp: false };
    this.voices.set(id, v);
    return v;
  }

  /** Updates every siren that should sound this frame; the rest fall silent. */
  update(dt: number, listener: { x: number; y: number; z: number; h: number }, sources: { id: string; x: number; y: number; z: number; yelp: boolean }[]) {
    const ctx = this.ctx;
    if (!ctx) return;
    const l = ctx.listener;
    const t = ctx.currentTime;
    if (l.positionX) {
      l.positionX.setTargetAtTime(listener.x, t, 0.05);
      l.positionY.setTargetAtTime(listener.y, t, 0.05);
      l.positionZ.setTargetAtTime(listener.z, t, 0.05);
      l.forwardX.setTargetAtTime(Math.sin(listener.h), t, 0.05);
      l.forwardZ.setTargetAtTime(-Math.cos(listener.h), t, 0.05);
    }
    const nearest = [...sources]
      .sort((a, b) => Math.hypot(a.x - listener.x, a.z - listener.z) - Math.hypot(b.x - listener.x, b.z - listener.z))
      .slice(0, 3);
    const live = new Set(nearest.map((s) => s.id));
    for (const [id, v] of this.voices) {
      if (!live.has(id)) {
        v.gain.gain.setTargetAtTime(0, t, 0.2);
        v.last = null;
        if (!sources.some((s) => s.id === id)) {
          v.osc.stop(t + 0.5);
          this.voices.delete(id);
        }
      }
    }
    for (const s of nearest) {
      const v = this.voice(s.id);
      if (!v) continue;
      const d = Math.hypot(s.x - listener.x, s.z - listener.z);
      let closing = 0;
      if (v.last && dt > 0) {
        const before = Math.hypot(v.last.x - listener.x, v.last.z - listener.z);
        closing = (before - d) / dt;
      }
      v.last = { x: s.x, z: s.z };
      const doppler = Math.min(1.25, Math.max(0.8, SOUND_SPEED / (SOUND_SPEED - Math.max(-120, Math.min(120, closing)))));
      v.phase += dt / (s.yelp ? 0.32 : 3.4);
      const sweep = s.yelp ? Math.abs(((v.phase % 1) * 2) - 1) : 0.5 + 0.5 * Math.sin(v.phase * Math.PI * 2);
      const f = (640 + 640 * sweep) * doppler;
      v.osc.frequency.setTargetAtTime(f, t, 0.02);
      v.pan.positionX.setTargetAtTime(s.x, t, 0.05);
      v.pan.positionY.setTargetAtTime(s.y + 1.6, t, 0.05);
      v.pan.positionZ.setTargetAtTime(s.z, t, 0.05);
      v.gain.gain.setTargetAtTime(this.muted ? 0 : 0.14, t, 0.1);
    }
  }

  /** A car alarm's horn pattern at a spot, for `secs` seconds. */
  alarm(x: number, z: number, secs: number, listener: { x: number; z: number }) {
    const ctx = this.ctx;
    if (!ctx || !this.out || this.muted) return;
    const d = Math.hypot(x - listener.x, z - listener.z);
    const level = Math.max(0, 1 - d / 400) * 0.12;
    if (level <= 0.002) return;
    const t = ctx.currentTime;
    const gain = ctx.createGain();
    gain.gain.value = 0;
    gain.connect(this.out);
    const beeps = Math.floor(secs / 0.6);
    for (let i = 0; i < beeps; i++) {
      gain.gain.setValueAtTime(level, t + i * 0.6);
      gain.gain.setValueAtTime(0, t + i * 0.6 + 0.32);
    }
    for (const f of [420, 525]) {
      const osc = ctx.createOscillator();
      osc.type = 'square';
      osc.frequency.value = f;
      osc.connect(gain);
      osc.start(t);
      osc.stop(t + secs);
    }
  }

  /** A short dog bark. */
  bark(x: number, z: number, listener: { x: number; z: number }) {
    const ctx = this.ctx;
    if (!ctx || !this.out || this.muted) return;
    const level = Math.max(0, 1 - Math.hypot(x - listener.x, z - listener.z) / 120) * 0.25;
    if (level <= 0.005) return;
    const t = ctx.currentTime;
    for (let i = 0; i < 2; i++) {
      const osc = ctx.createOscillator();
      const gain = ctx.createGain();
      osc.type = 'sawtooth';
      osc.frequency.setValueAtTime(380, t + i * 0.28);
      osc.frequency.exponentialRampToValueAtTime(160, t + i * 0.28 + 0.14);
      gain.gain.setValueAtTime(level, t + i * 0.28);
      gain.gain.exponentialRampToValueAtTime(0.0001, t + i * 0.28 + 0.16);
      osc.connect(gain).connect(this.out);
      osc.start(t + i * 0.28);
      osc.stop(t + i * 0.28 + 0.2);
    }
  }

  dispose() {
    for (const v of this.voices.values()) v.osc.stop();
    this.voices.clear();
  }
}
