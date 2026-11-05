import type { Instrument } from './types';

/**
 * How long after a remote player's cue its song time is applied. The cue
 * crosses the network first, so everyone schedules it this far behind and
 * hears the dropout at the same point in the song.
 */
export const CUE_HOLD_MS = 150;

type Seat = { instrument: Instrument; out: boolean; t: number; cued: boolean };

/**
 * Decides when each part's stem drops out of the mix. A part played by more
 * than one player drops only while every one of them is out.
 */
export class StemGate {
  private seats = new Map<number, Seat>();
  private parts = new Map<Instrument, boolean>();

  constructor(private apply: (instrument: Instrument, out: boolean, atSongMs: number) => void) {}

  /**
   * Follows who is playing what. Until a player sends cues, the out state
   * in their progress stands in, applied at `now`.
   */
  roster(players: { conn: number; instrument: Instrument; out: boolean }[], now: number) {
    const present = new Set(players.map((p) => p.conn));
    for (const conn of [...this.seats.keys()]) if (!present.has(conn)) this.seats.delete(conn);
    for (const player of players) {
      const seat = this.seats.get(player.conn);
      if (!seat || seat.instrument !== player.instrument) {
        this.seats.set(player.conn, { instrument: player.instrument, out: player.out, t: -1e9, cued: false });
      } else if (!seat.cued) seat.out = player.out;
    }
    for (const instrument of new Set(players.map((p) => p.instrument))) this.settle(instrument, now);
  }

  /** A player dropped out or came back at song time `t`; their stem follows `hold` later. */
  cue(conn: number, t: number, out: boolean, hold: number) {
    const seat = this.seats.get(conn);
    if (!seat || t < seat.t) return;
    seat.t = t;
    seat.cued = true;
    if (seat.out === out) return;
    seat.out = out;
    this.settle(seat.instrument, t + hold);
  }

  isOut(instrument: Instrument) {
    return this.parts.get(instrument) ?? false;
  }

  private settle(instrument: Instrument, at: number) {
    const seats = [...this.seats.values()].filter((s) => s.instrument === instrument);
    const out = seats.length > 0 && seats.every((s) => s.out);
    if ((this.parts.get(instrument) ?? false) === out) return;
    this.parts.set(instrument, out);
    this.apply(instrument, out, at);
  }
}

const OPEN_MS = 40;
const CLOSE_MS = 150;

/**
 * Whether a singer is on the note right now, from their pitch readings. It
 * opens after a short run on pitch and closes after a short run off it, so a
 * single bad reading does not flutter the vocals.
 */
export class SingingGate {
  out = false;
  changedAt = -1e9;
  private since = -1e9;
  private pending: boolean | null = null;

  /** `on` is whether this reading is on the current note; `null` means no note is due. */
  sample(time: number, on: boolean | null) {
    if (on === null || on !== this.out) {
      this.pending = null;
      return false;
    }
    const want = !on;
    if (this.pending !== want) {
      this.pending = want;
      this.since = time;
    }
    if (time - this.since < (want ? CLOSE_MS : OPEN_MS)) return false;
    this.out = want;
    this.changedAt = time;
    this.pending = null;
    return true;
  }
}
