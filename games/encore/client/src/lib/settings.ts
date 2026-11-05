import { DEFAULT_BINDINGS, cleanBindings, type Bindings, type Control, type Layout } from './input';

export type Settings = {
  /** The audio offset: how late this player's presses land after the sound, in milliseconds; subtracted when judging. */
  inputOffset: number;
  /** The video offset: shifts only the notes on screen; higher makes them reach the line later. */
  videoOffset: number;
  /** Nudges the audio offset after each song from how the hits landed. */
  autoAdjust: boolean;
  /** How far ahead the highway shows, in milliseconds. */
  lookahead: number;
  volume: number;
  /** Loudness of the drum sounds each pad hit makes; 0 turns them off. */
  drumVolume: number;
  autoKick: boolean;
  /** A vocalist hears the real vocals follow their singing. */
  vocalGuide: boolean;
  /** Mutes this player's lobby voice while they sing, so the room hears the song rather than them. */
  muteLobbyWhileSinging: boolean;
  bindings: Record<Layout, Bindings>;
};

export const DEFAULT_SETTINGS: Settings = {
  inputOffset: 0,
  videoOffset: 0,
  autoAdjust: false,
  lookahead: 1100,
  volume: 0.9,
  drumVolume: 0.8,
  autoKick: false,
  vocalGuide: true,
  muteLobbyWhileSinging: true,
  bindings: DEFAULT_BINDINGS,
};

const clamp = (value: unknown, min: number, max: number, fallback: number) =>
  typeof value === 'number' && Number.isFinite(value) ? Math.min(max, Math.max(min, value)) : fallback;

/** Reads settings the server kept, filling anything missing or malformed with defaults. */
export function readSettings(raw: unknown): Settings {
  const from = (raw && typeof raw === 'object' ? raw : {}) as Partial<Settings>;
  const bindings = {} as Record<Layout, Bindings>;
  for (const layout of ['guitar', 'drums'] as Layout[]) {
    const saved = (from.bindings?.[layout] ?? {}) as Partial<Bindings>;
    bindings[layout] = { ...DEFAULT_BINDINGS[layout] };
    for (const control of Object.keys(DEFAULT_BINDINGS[layout]) as Control[]) {
      const clean = cleanBindings(saved[control]);
      if (clean) bindings[layout][control] = clean;
    }
  }
  return {
    inputOffset: clamp(from.inputOffset, -300, 300, 0),
    videoOffset: clamp(from.videoOffset, -300, 300, 0),
    autoAdjust: typeof from.autoAdjust === 'boolean' ? from.autoAdjust : DEFAULT_SETTINGS.autoAdjust,
    lookahead: clamp(from.lookahead, 500, 2500, DEFAULT_SETTINGS.lookahead),
    volume: clamp(from.volume, 0, 1, DEFAULT_SETTINGS.volume),
    drumVolume: clamp(from.drumVolume, 0, 1, DEFAULT_SETTINGS.drumVolume),
    autoKick: typeof from.autoKick === 'boolean' ? from.autoKick : DEFAULT_SETTINGS.autoKick,
    vocalGuide: typeof from.vocalGuide === 'boolean' ? from.vocalGuide : DEFAULT_SETTINGS.vocalGuide,
    muteLobbyWhileSinging:
      typeof from.muteLobbyWhileSinging === 'boolean' ? from.muteLobbyWhileSinging : DEFAULT_SETTINGS.muteLobbyWhileSinging,
    bindings,
  };
}
