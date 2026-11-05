/** Shared with the voice client in the tawks app. */
export const SINGING_EVENT = "tawks:singing";

type Timers = {
  set: (fn: () => void, ms: number) => unknown;
  clear: (handle: unknown) => void;
};

/**
 * Holds a player's lobby mic muted while their game says they are singing.
 * It lets go when the game says stop, when the frame goes away, or when the
 * game's heartbeat stops, so nobody is ever left muted.
 */
export class SingingHold {
  on = false;
  private timer: unknown = null;
  private emit: (on: boolean) => void;
  private timeoutMs: number;
  private timers: Timers;

  constructor(
    emit: (on: boolean) => void,
    timeoutMs: number,
    timers: Timers = {
      set: (fn, ms) => setTimeout(fn, ms),
      clear: (handle) => clearTimeout(handle as ReturnType<typeof setTimeout>),
    },
  ) {
    this.emit = emit;
    this.timeoutMs = timeoutMs;
    this.timers = timers;
  }

  beat(on: boolean) {
    if (!on) return this.release();
    if (this.timer !== null) this.timers.clear(this.timer);
    this.timer = this.timers.set(() => this.release(), this.timeoutMs);
    if (this.on) return;
    this.on = true;
    this.emit(true);
  }

  release() {
    if (this.timer !== null) this.timers.clear(this.timer);
    this.timer = null;
    if (!this.on) return;
    this.on = false;
    this.emit(false);
  }
}
