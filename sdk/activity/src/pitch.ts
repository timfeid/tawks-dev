import type { PitchSample } from './protocol';

const WINDOW = 1024;
const MIN_HZ = 70;
const MAX_HZ = 1100;
/** YIN's aperiodicity threshold: below it a frame counts as pitched. */
const THRESHOLD = 0.15;
const SILENCE = 0.008;
const SEND_EVERY_MS = 33;

/**
 * Listens to the microphone and reports what is being sung, pitch and
 * loudness only. Audio processing that suits speech (echo cancellation, noise
 * suppression) is off, since it smears sustained notes.
 */
export class PitchTracker {
  private stream: MediaStream | null = null;
  private ctx: AudioContext | null = null;
  private timer: ReturnType<typeof setInterval> | null = null;
  private pending: PitchSample[] = [];
  private lastSent = 0;

  constructor(private onSamples: (samples: PitchSample[]) => void) {}

  async start() {
    this.stream = await navigator.mediaDevices.getUserMedia({
      audio: { echoCancellation: false, noiseSuppression: false, autoGainControl: false },
    });
    this.ctx = new AudioContext({ latencyHint: 'interactive' });
    // A context made without a fresh click starts suspended and would hear only silence.
    await this.ctx.resume();
    const source = this.ctx.createMediaStreamSource(this.stream);
    const analyser = this.ctx.createAnalyser();
    analyser.fftSize = WINDOW * 2;
    source.connect(analyser);
    const buffer = new Float32Array(analyser.fftSize);
    const rate = this.ctx.sampleRate;
    // The newest sample in the buffer is about now; the window centre is earlier.
    const centreMs = (analyser.fftSize / 2 / rate) * 1000;

    this.timer = setInterval(() => {
      analyser.getFloatTimeDomainData(buffer);
      const t = performance.timeOrigin + performance.now() - centreMs;
      const level = rms(buffer);
      const hz = level < SILENCE ? null : yin(buffer, rate);
      this.pending.push({ t: Math.round(t), hz: hz === null ? null : Math.round(hz * 10) / 10, level: Math.round(level * 1000) / 1000 });
      const now = performance.now();
      if (now - this.lastSent >= SEND_EVERY_MS) {
        this.lastSent = now;
        this.onSamples(this.pending.splice(0));
      }
    }, 16);
  }

  stop() {
    if (this.timer) clearInterval(this.timer);
    this.timer = null;
    this.stream?.getTracks().forEach((track) => track.stop());
    this.stream = null;
    void this.ctx?.close();
    this.ctx = null;
  }
}

function rms(buffer: Float32Array) {
  let sum = 0;
  for (let i = 0; i < buffer.length; i++) sum += buffer[i] * buffer[i];
  return Math.sqrt(sum / buffer.length);
}

/** de Cheveigné and Kawahara's YIN, over the first WINDOW samples. */
function yin(buffer: Float32Array, rate: number): number | null {
  const minTau = Math.floor(rate / MAX_HZ);
  const maxTau = Math.min(WINDOW - 1, Math.ceil(rate / MIN_HZ));
  const diff = new Float32Array(maxTau + 1);
  for (let tau = 1; tau <= maxTau; tau++) {
    let sum = 0;
    for (let i = 0; i < WINDOW; i++) {
      const d = buffer[i] - buffer[i + tau];
      sum += d * d;
    }
    diff[tau] = sum;
  }
  let running = 0;
  for (let tau = 1; tau <= maxTau; tau++) {
    running += diff[tau];
    diff[tau] = running === 0 ? 1 : (diff[tau] * tau) / running;
  }
  for (let tau = minTau; tau <= maxTau; tau++) {
    if (diff[tau] < THRESHOLD) {
      while (tau + 1 <= maxTau && diff[tau + 1] < diff[tau]) tau++;
      const better = tau > 1 && tau < maxTau ? parabola(diff, tau) : tau;
      return rate / better;
    }
  }
  return null;
}

function parabola(values: Float32Array, i: number) {
  const a = values[i - 1];
  const b = values[i];
  const c = values[i + 1];
  const shift = (a - c) / (2 * (a - 2 * b + c));
  return Number.isFinite(shift) ? i + shift : i;
}
