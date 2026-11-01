const WINDOW = 40;
const RECENT = 10;
const MIN_DELAY = 0.1;
const MAX_DELAY = 0.25;
const SLEW = 0.15;
const RESYNC = 0.5;

/**
 * Picks the server moment to draw so there is nearly always a snapshot on
 * either side of it. The delay is measured against the quickest arrival of the
 * last half second, so a server that runs a little slow or fast is followed
 * instead of drifting into constant extrapolation, and it widens when packets
 * arrive unevenly. Playback changes speed by a few percent while it catches up
 * rather than jumping.
 */
export class SnapshotClock {
  private offsets: number[] = [];
  private target = 0;
  private lag: number | null = null;
  private last: number | null = null;

  reset() {
    this.offsets = [];
    this.lag = null;
    this.last = null;
  }

  arrive(serverTime: number, now: number) {
    this.offsets.push(now - serverTime);
    if (this.offsets.length > WINDOW) this.offsets.shift();
    const sorted = [...this.offsets].sort((a, b) => a - b);
    const late = sorted[Math.floor((sorted.length - 1) * 0.9)] - sorted[0];
    const quickest = Math.min(...this.offsets.slice(-RECENT));
    this.target = quickest + Math.min(MAX_DELAY, Math.max(MIN_DELAY, late + 0.03));
  }

  renderTime(now: number): number | null {
    if (!this.offsets.length) return null;
    const step = this.last === null ? 0 : Math.max(0, now - this.last);
    this.last = now;
    const gap = this.target - (this.lag ?? this.target);
    if (this.lag === null || Math.abs(gap) > RESYNC) this.lag = this.target;
    else this.lag += Math.max(-SLEW * step, Math.min(SLEW * step, gap));
    return now - this.lag;
  }
}
