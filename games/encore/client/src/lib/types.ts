export type Instrument = 'guitar' | 'bass' | 'rhythm' | 'keys' | 'drums' | 'vocals';
export type Difficulty = 'easy' | 'medium' | 'hard' | 'expert';
export type NoteKind = 'strum' | 'hopo' | 'tap';

export const INSTRUMENTS: Instrument[] = ['guitar', 'bass', 'rhythm', 'keys', 'drums', 'vocals'];
export const DIFFICULTIES: Difficulty[] = ['easy', 'medium', 'hard', 'expert'];

export const INSTRUMENT_LABELS: Record<Instrument, string> = {
  guitar: 'Guitar',
  bass: 'Bass',
  rhythm: 'Rhythm',
  keys: 'Keys',
  drums: 'Drums',
  vocals: 'Vocals',
};

/** Lane bits 0 to 4 are frets green to orange, or the kick then four pads on drums. */
export const OPEN = 1 << 5;

export type Note = { t: number; len: number; lanes: number; kind: NoteKind };

export type Track = {
  instrument: Instrument;
  difficulty: Difficulty;
  notes: Note[];
  star_power: [number, number][];
};

/** A note to sing: `pitch` is a MIDI note number; a talky note takes any voice. */
export type VocalNote = { t: number; len: number; pitch: number; talky: boolean };

export type LyricLine = { start: number; end: number; words: { t: number; text: string; joins: boolean }[] };

export type Chart = {
  tracks: Track[];
  beats: { t: number; bar: boolean }[];
  sections: { t: number; name: string }[];
  length: number;
  lyrics: LyricLine[];
  vocals: VocalNote[];
};

export type SongInfo = {
  id: string;
  name: string;
  artist: string;
  album: string | null;
  year: string | null;
  genre: string | null;
  charter: string | null;
  length_ms: number;
  preview_ms: number;
  parts: { instrument: Instrument; intensity: number | null }[];
  stems: string[];
  art: boolean;
  /** `library` is the server's own folder; `upload` is a player's song. */
  source?: 'library' | 'upload';
  /** Licensed or original; only these songs earn rewards. */
  official?: boolean;
  uploaded_by?: string | null;
  uploader_name?: string | null;
};

export type UploaderStatus = { can_upload: boolean; reason: string | null; songs: number; max_songs: number };

/** What `/me` says about the player's access. */
export type Me = {
  library: boolean;
  uploads: boolean;
  status: UploaderStatus | null;
  limits: { max_total_mb: number; max_audio_mb: number; max_minutes: number; formats: string[] };
};

export const REPORT_REASONS = [
  { key: 'copyright', label: 'It uses music the uploader has no right to' },
  { key: 'offensive', label: 'It is offensive or abusive' },
  { key: 'broken', label: 'The chart or audio is broken' },
  { key: 'other', label: 'Something else' },
] as const;

export type ReportReason = (typeof REPORT_REASONS)[number]['key'];

export type PartChoice = { instrument: Instrument; difficulty: Difficulty };

export type Progress = {
  score: number;
  streak: number;
  best_streak: number;
  multiplier: number;
  hit: number;
  missed: number;
  total: number;
  star_meter: number;
  star_active: boolean;
  muted: boolean;
};

export type Player = {
  conn: number;
  user_id: string;
  name: string;
  avatar_url: string;
  part: PartChoice | null;
  ready: boolean;
  loaded: boolean;
  playing: boolean;
  done: boolean;
  progress: Progress;
  away: boolean;
};

export type Phase = 'lobby' | 'loading' | 'playing' | 'results';

export type RoomView = {
  phase: Phase;
  song: SongInfo | null;
  picked_by: string | null;
  host: number | null;
  start_at: number | null;
  ends_at: number | null;
  players: Player[];
};

export type ServerMsg =
  | {
      type: 'welcome';
      conn: number;
      user_id: string;
      settings: unknown;
      bests: Record<string, number>;
      library: boolean;
      uploads: boolean;
    }
  | { type: 'room'; room: RoomView }
  | { type: 'pong'; t: number; now: number }
  | { type: 'error'; message: string };

export type ClientMsg =
  | { type: 'hello'; token: string }
  | { type: 'ping'; t: number }
  | { type: 'pick'; song_id: string }
  | { type: 'part'; instrument: Instrument | null; difficulty: Difficulty | null }
  | { type: 'ready'; ready: boolean }
  | { type: 'loaded' }
  | { type: 'progress'; progress: Progress; done: boolean }
  | { type: 'again' }
  | { type: 'stop' }
  | { type: 'settings'; settings: unknown };

/** What the join action hands the frame. */
export type Pass = { ws_url: string; api_url: string; token: string; lobby_id: string };

export function bestKey(songId: string, part: PartChoice) {
  return `${songId}:${part.instrument}:${part.difficulty}`;
}

export function formatLength(ms: number) {
  const seconds = Math.round(ms / 1000);
  return `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, '0')}`;
}
