import { glide } from "../audio-param";
/** CC0 recordings, with their primary sources and processing in sounds/city/CREDITS.json.
 * A bounded audio graph: one bed, two distant crowds, one local building hum,
 * one wind voice and an occasional distant siren. No per-frame audio allocations. */
const FILES = import.meta.glob("../sounds/city/*.ogg", {
  query: "?url",
  import: "default",
  eager: true,
}) as Record<string, string>;
type Voice = {
  source: AudioBufferSourceNode;
  gain: GainNode;
  pan: StereoPannerNode;
};
export class CityAmbience {
  private buffers = new Map<string, AudioBuffer>();
  private loading: Promise<void> | null = null;
  private voices = new Map<string, Voice>();
  private wind: Voice | null = null;
  private event: Voice | null = null;
  private eventIn = 12;
  private enabled = false;
  private elapsed = 0;
  private mixing = 0;
  constructor(
    private ctx: AudioContext,
    private out: AudioNode,
  ) {}
  private load() {
    if (!this.loading)
      this.loading = Promise.all(
        Object.entries(FILES).map(async ([path, url]) => {
          try {
            const bytes = await fetch(url).then((r) => {
              if (!r.ok) throw Error("Ambience unavailable");
              return r.arrayBuffer();
            });
            this.buffers.set(
              path.split("/").at(-1)!.replace(".ogg", ""),
              await this.ctx.decodeAudioData(bytes),
            );
          } catch {
            /* No retry loop or stalled play if an optional ambience file is unavailable. */
          }
        }),
      ).then(() => {});
  }
  private voice(key: string, recording: string, rate = 1): Voice | null {
    const existing = this.voices.get(key);
    if (existing) return existing;
    const buffer = this.buffers.get(recording);
    if (!buffer) return null;
    const voice = this.make(buffer, true, rate);
    this.voices.set(key, voice);
    return voice;
  }
  private make(buffer: AudioBuffer, loop: boolean, rate = 1): Voice {
    const source = this.ctx.createBufferSource(),
      gain = this.ctx.createGain(),
      pan = this.ctx.createStereoPanner();
    source.buffer = buffer;
    source.loop = loop;
    source.playbackRate.value = rate;
    gain.gain.value = 0;
    source.connect(gain).connect(pan).connect(this.out);
    source.start(
      0,
      loop ? Math.random() * Math.max(0.1, buffer.duration - 1) : 0,
    );
    return { source, gain, pan };
  }
  private levels(voice: Voice | null, volume: number, pan: number) {
    if (!voice) return;
    glide(voice.gain.gain, volume, this.ctx.currentTime, 0.3);
    glide(
      voice.pan.pan,
      Math.max(-0.85, Math.min(0.85, pan)),
      this.ctx.currentTime,
      0.25,
    );
  }
  frame(
    dt: number,
    listener: { x: number; z: number; yaw: number },
    enabled: boolean,
    volume: number,
  ) {
    this.enabled = enabled;
    this.elapsed += dt;
    if (!enabled) {
      this.stop();
      return;
    }
    this.load();
    this.mixing +=
      (Math.max(0, Math.min(1, volume)) - this.mixing) *
      (1 - Math.exp(-dt * 1.8));
    const scale = this.mixing;
    this.levels(
      this.voice("bed", "street"),
      scale * 0.48,
      Math.sin(this.elapsed * 0.035) * 0.15,
    );
    // Actual indoor crowd walla belongs behind shop fronts rather than following the camera.
    const crowds = [
      [-22, 34],
      [34, -34],
      [-34, -22],
      [22, 34],
    ];
    crowds.sort(
      (a, b) =>
        Math.hypot(a[0] - listener.x, a[1] - listener.z) -
        Math.hypot(b[0] - listener.x, b[1] - listener.z),
    );
    const spatial = (x: number, z: number, radius: number) => {
      const dx = x - listener.x,
        dz = z - listener.z,
        d = Math.hypot(dx, dz);
      return {
        volume: Math.max(0, 1 - d / radius) ** 2,
        pan:
          (dx * Math.cos(listener.yaw) - dz * Math.sin(listener.yaw)) /
          Math.max(4, d),
      };
    };
    for (let i = 0; i < 2; i++) {
      const point = crowds[i],
        space = spatial(point[0], point[1], 38);
      this.levels(
        this.voice("crowd" + i, "crowd", i ? 0.94 : 1.03),
        scale * 0.55 * space.volume,
        space.pan,
      );
    }
    const hum = spatial(22, 6, 32);
    this.levels(this.voice("hum", "hum"), scale * 0.38 * hum.volume, hum.pan);
    if (!this.wind) {
      const buffer = this.ctx.createBuffer(
        1,
        this.ctx.sampleRate * 3,
        this.ctx.sampleRate,
      );
      const data = buffer.getChannelData(0);
      let smooth = 0;
      for (let i = 0; i < data.length; i++) {
        smooth = (smooth + (Math.random() * 2 - 1) * 0.035) / 1.035;
        data[i] = smooth;
      }
      this.wind = this.make(buffer, true);
    }
    this.levels(
      this.wind,
      scale * (0.045 + 0.015 * Math.sin(this.elapsed * 0.18)),
      Math.sin(this.elapsed * 0.07) * 0.45,
    );
    this.eventIn -= dt;
    if (this.eventIn <= 0 && !this.event) {
      const buffer = this.buffers.get("siren");
      if (buffer) {
        const voice = this.make(buffer, false);
        this.event = voice;
        this.levels(voice, scale * 0.1, -0.7);
        voice.source.onended = () => {
          voice.gain.disconnect();
          voice.pan.disconnect();
          if (this.event === voice) this.event = null;
        };
      }
      this.eventIn = 38 + Math.random() * 30;
    }
    if (this.event)
      glide(
        this.event.pan.pan,
        Math.sin(this.elapsed * 0.12) * 0.7,
        this.ctx.currentTime,
        0.3,
      );
  }
  stop() {
    if (!this.voices.size && !this.wind && !this.event) return;
    for (const voice of [...this.voices.values(), this.wind, this.event]) {
      if (!voice) continue;
      try {
        voice.source.stop();
      } catch {}
      voice.gain.disconnect();
      voice.pan.disconnect();
    }
    this.voices.clear();
    this.wind = null;
    this.event = null;
    this.mixing = 0;
  }
  stats() {
    return {
      decoded: this.buffers.size,
      loops: this.voices.size + (this.wind ? 1 : 0),
      events: this.event ? 1 : 0,
    };
  }
  dispose() {
    this.stop();
    this.buffers.clear();
  }
}
