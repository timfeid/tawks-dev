import type { BoutModule } from '@tawks/bout-sdk';
import type { GameEvent, GameSnap, RoomView } from './protocol';

type Blender = Pick<BoutModule, 'blend'>;
type Point = { x: number; y: number; z: number };
type Frame = { at: number; game: GameSnap; bytes: number };
type Event = { at: number; event: GameEvent };
type Highlight = { at: number; rank: number; label: string; seat: number | null; point: Point | null };
type Cue = { readyAt: number; deadline: number };
type Plan = {
  cue: Highlight;
  startedAt: number;
  deadline: number;
  source: number[];
  wall: number[];
  previous: number;
  elapsed: number;
  speed: number;
};

export type ReplayStatus = {
  phase: 'pending' | 'playing';
  label: string;
  speed: number;
  progress: number;
};

export type ReplaySample = {
  snapshot: GameSnap;
  /** Historical effects crossed on this frame, emitted once in replay order. */
  events: GameEvent[];
  time: number;
  dt: number;
  speed: number;
  progress: number;
  label: string;
  focusSeat: number | null;
  focusPoint: Point | null;
};

export type BoardReplaySample = Omit<ReplaySample, 'snapshot' | 'events'> & { room: RoomView };

export const REPLAY_LIMITS = {
  historySeconds: 8,
  frames: 200,
  bytes: 8 * 1024 * 1024,
  events: 800,
  candidates: 80,
  slowest: 0.2,
  /** Preserve a clear result screen before the next round starts. */
  resultReserve: 1,
  /** The final snapshot/events can follow the room's results message. */
  settleSeconds: 0.15,
} as const;

const ease = (t: number) => {
  const x = Math.max(0, Math.min(1, t));
  return x * x * (3 - 2 * x);
};

/** Continuous source-time ramp, including zero slope at the 20% plateau. */
export function replaySpeed(secondsFromImpact: number, rampEnd = 0.42) {
  if (secondsFromImpact < -0.55) return 1;
  if (secondsFromImpact < -0.12) return 1 - 0.8 * ease((secondsFromImpact + 0.55) / 0.43);
  if (secondsFromImpact <= 0.08) return REPLAY_LIMITS.slowest;
  const end = Math.max(0.09, rampEnd);
  if (secondsFromImpact < end) return REPLAY_LIMITS.slowest + 0.8 * ease((secondsFromImpact - 0.08) / (end - 0.08));
  return 1;
}

const keyOf = (game: GameSnap) => game.game ?? 'bumper';
const number = (value: unknown): value is number => typeof value === 'number' && Number.isFinite(value);
const point = (value: unknown): Point | null => {
  if (!value || typeof value !== 'object') return null;
  const p = value as Record<string, unknown>;
  return number(p.x) && number(p.z) ? { x: p.x, y: number(p.y) ? p.y : 0, z: p.z } : null;
};

function playerPoint(game: GameSnap, seat: number | null) {
  if (seat === null) return null;
  const raw = game as unknown as Record<string, unknown>;
  const rows = Array.isArray(raw.players) ? raw.players : Array.isArray(raw.balls) ? raw.balls : [];
  return point(rows.find((row: { seat?: number }) => row.seat === seat));
}

function finalElimination(game: GameSnap) {
  const raw = game as unknown as Record<string, unknown>;
  const rows = Array.isArray(raw.players) ? raw.players : Array.isArray(raw.balls) ? raw.balls : [];
  if (rows.length < 2) return false;
  const hasState = rows.some((row) => 'alive' in row || 'out' in row || 'state' in row);
  return hasState && rows.filter((row) => row.alive !== false && row.out !== true && row.state !== 'out' && row.state !== 'falling').length <= 1;
}

function highlight(game: GameSnap, event: GameEvent, at: number): Highlight | null {
  const e = event as unknown as Record<string, unknown>;
  // Focus the target/impact rather than a distant shooter. In elimination
  // events `seat` is the victim, while `by` is the attacker.
  const seat = number(e.target) && e.target >= 0 ? e.target : number(e.seat) && e.seat >= 0 ? e.seat : number(e.by) && e.by >= 0 ? e.by : null;
  const location = point(e) ?? playerPoint(game, seat);
  const make = (rank: number, label: string): Highlight => ({ at, rank, label, seat, point: location });
  switch (e.kind) {
    case 'titan_down': return make(130, 'The final blow');
    case 'finish': return make(e.place === 1 ? 120 : 85, e.place === 1 ? 'Winning finish' : 'Across the line');
    case 'capture': return make(115, 'Objective secured');
    case 'round_end': return make(110, 'Round-winning moment');
    case 'goal': return make(110, 'The winning play');
    case 'defuse': return number(e.progress) && e.progress >= 1 ? make(110, 'Clutch defuse') : null;
    case 'splat': return e.down === true ? make(finalElimination(game) ? 120 : 95, 'The finishing shot') : null;
    case 'down': return make(finalElimination(game) ? 120 : 90, 'The finishing shot');
    case 'out':
    case 'wreck':
    case 'burst':
    case 'boom':
    case 'fall':
    case 'drop':
    case 'off':
    case 'over':
    case 'floored':
    case 'dunked': return make(finalElimination(game) ? 120 : 80, finalElimination(game) ? 'Last one standing' : 'The knockout');
    case 'catch': return make(65, 'Clutch catch');
    case 'target_hit': return make(50 + Math.min(20, number(e.points) ? e.points : 0), 'Right on target');
    default: return null;
  }
}

/**
 * Local, presentation-only highlight playback. It never writes a room, changes
 * a game clock, or sends input. Only results may play a replay; live players
 * always retain the real view and controls.
 */
export class ReplayDirector {
  private readonly board: BoardReplayDirector;
  private frames: Frame[] = [];
  private events: Event[] = [];
  private highlights: Highlight[] = [];
  private retainedBytes = 0;
  private key = '';
  private phase = '';
  private cue: Cue | null = null;
  private plan: Plan | null = null;
  private lastSourceTime: number | null = null;

  private readonly now: () => number;

  constructor(now: () => number = () => performance.now() / 1000) { this.now = now; this.board = new BoardReplayDirector(now); }

  get active() { return this.plan !== null || this.board.active; }
  get pending() { return this.cue !== null || this.board.pending; }
  get status(): ReplayStatus | null {
    const p = this.plan;
    if (p) return { phase: 'playing', label: p.cue.label, speed: p.speed, progress: p.elapsed / p.wall[p.wall.length - 1] };
    return this.cue ? { phase: 'pending', label: 'Highlight replay', speed: 1, progress: 0 } : this.board.status;
  }

  /** Exposed for memory-budget diagnostics, without exposing mutable history. */
  get retained() { return { frames: this.frames.length, events: this.events.length, bytes: this.retainedBytes }; }

  reset() {
    this.board.reset();
    this.clearHistory();
    this.key = '';
    this.phase = '';
  }

  setViewer(userId: string) { this.board.setViewer(userId); }

  skip() { this.stopMinigame(); this.board.skip(); }

  private stopMinigame() { this.cue = null; this.plan = null; }

  private clearHistory() {
    this.frames = [];
    this.events = [];
    this.highlights = [];
    this.retainedBytes = 0;
    this.lastSourceTime = null;
    this.stopMinigame();
  }

  /** Called before the real room callback, so a next-phase packet cancels immediately. */
  observeRoom(room: Pick<RoomView, 'minigame' | 'phase'> & Partial<RoomView>) {
    this.board.observe(room);
    const key = room.minigame;
    const phase = room.phase.name;
    const changed = key !== this.key;
    if (changed || (phase === 'playing' && this.phase !== 'playing')) this.clearHistory();
    if (phase !== 'results') this.stopMinigame();
    if (phase === 'results') {
      const now = this.now();
      const deadline = now + Math.max(0, room.phase.name === 'results' ? room.phase.seconds - REPLAY_LIMITS.resultReserve : 0);
      if (!changed && this.phase !== 'results' && this.phase === 'playing' && deadline - now > 1.2) {
        this.cue = { readyAt: now + REPLAY_LIMITS.settleSeconds, deadline };
      }
      // Repeated timer packets may shorten a replay's budget, never extend it.
      if (this.cue) this.cue.deadline = Math.min(this.cue.deadline, deadline);
      if (this.plan) this.plan.deadline = Math.min(this.plan.deadline, deadline);
    }
    this.key = key;
    this.phase = phase;
  }

  record(serverTime: number, snapshot: GameSnap, packetEvents: GameEvent[], encodedBytes?: number) {
    if (!number(serverTime) || !['playing', 'results'].includes(this.phase) || keyOf(snapshot) !== this.key) return;
    const last = this.frames[this.frames.length - 1];
    const clock = (snapshot as unknown as { time?: number }).time;
    if (last && (serverTime < last.at || (number(clock) && this.lastSourceTime !== null && clock < this.lastSourceTime - 0.25))) this.clearHistory();
    const bytes = encodedBytes ?? JSON.stringify([snapshot, packetEvents]).length * 2;
    if (!number(bytes) || bytes < 0 || bytes > REPLAY_LIMITS.bytes) { this.clearHistory(); return; }
    if (number(clock)) this.lastSourceTime = clock;
    const copy = structuredClone(snapshot);
    const latest = this.frames[this.frames.length - 1];
    if (latest?.at === serverTime) {
      this.retainedBytes -= latest.bytes;
      this.frames.pop();
    }
    this.frames.push({ at: serverTime, game: copy, bytes });
    this.retainedBytes += bytes;
    for (const original of packetEvents) {
      const event = structuredClone(original);
      // Final room/snapshot packets can repeat one tick while joining/reconnecting.
      if (this.events.some((old) => old.at === serverTime && JSON.stringify(old.event) === JSON.stringify(event))) continue;
      this.events.push({ at: serverTime, event });
      if (this.phase === 'playing' || this.cue) {
        const candidate = highlight(copy, event, serverTime);
        if (candidate) this.highlights.push(candidate);
      }
    }
    while (this.frames.length > 1 && (this.frames.length > REPLAY_LIMITS.frames || this.retainedBytes > REPLAY_LIMITS.bytes || this.frames[0].at < serverTime - REPLAY_LIMITS.historySeconds)) {
      this.retainedBytes -= this.frames.shift()!.bytes;
    }
    const oldest = this.frames[0]?.at ?? serverTime;
    this.events = this.events.filter((one) => one.at >= oldest).slice(-REPLAY_LIMITS.events);
    this.highlights = this.highlights.filter((one) => one.at >= oldest).slice(-REPLAY_LIMITS.candidates);
  }

  private begin(cue: Cue, now: number) {
    this.cue = null;
    if (this.frames.length < 2 || cue.deadline - now < 1.2) return;
    const first = this.frames[0].at;
    const last = this.frames[this.frames.length - 1].at;
    const best = this.highlights
      .filter((one) => one.at - first >= 0.35 && last - one.at <= 6)
      .sort((a, b) => b.rank - a.rank || b.at - a.at)[0];
    if (!best) return;
    const start = Math.max(first, best.at - 1.15);
    const end = Math.min(last, best.at + 0.4);
    if (end - start < 0.35) return;
    const source = [start];
    const wall = [0];
    for (let at = start; at < end - 1e-7;) {
      const next = Math.min(end, at + 1 / 240);
      const seconds = (next - at) / replaySpeed((at + next) * 0.5 - best.at, end - best.at);
      source.push(next);
      wall.push(wall[wall.length - 1] + seconds);
      at = next;
    }
    // Shorten only normal-speed lead-in; never rush the hit or overrun results.
    const budget = cue.deadline - now;
    while (wall[wall.length - 1] > budget && source.length > 2 && source[1] < best.at - 0.55) {
      source.shift();
      const removed = wall[1];
      wall.shift();
      for (let i = 0; i < wall.length; i++) wall[i] -= removed;
    }
    if (wall[wall.length - 1] > budget || wall[wall.length - 1] < 0.8) return;
    this.plan = { cue: best, startedAt: now, deadline: cue.deadline, source, wall, previous: source[0] - 1e-7, elapsed: 0, speed: 1 };
  }

  /** Render this instead of the live snapshot, never instead of the live room. */
  sample(bout: Blender | null, key: string, now = this.now()): ReplaySample | null {
    if (this.phase !== 'results' || key !== this.key) { this.stopMinigame(); return null; }
    if (this.cue && now >= this.cue.readyAt) this.begin(this.cue, now);
    const p = this.plan;
    if (!p) return null;
    const elapsed = Math.max(0, now - p.startedAt);
    const duration = p.wall[p.wall.length - 1];
    if (now >= p.deadline || elapsed > duration + 1e-6) { this.skip(); return null; }
    let lo = 0, hi = p.wall.length - 1;
    while (hi - lo > 1) {
      const mid = (lo + hi) >> 1;
      if (p.wall[mid] <= elapsed) lo = mid; else hi = mid;
    }
    const blend = Math.max(0, Math.min(1, (elapsed - p.wall[lo]) / Math.max(1e-7, p.wall[hi] - p.wall[lo])));
    const sourceTime = p.source[lo] + (p.source[hi] - p.source[lo]) * blend;
    const snapshot = this.at(bout, key, sourceTime);
    if (!snapshot) { this.skip(); return null; }
    const effects = this.events.filter((one) => one.at > p.previous && one.at <= sourceTime + 1e-7).map((one) => structuredClone(one.event));
    const dt = Math.max(0, Math.min(0.1, sourceTime - Math.max(p.source[0], p.previous)));
    p.previous = Math.max(p.previous, sourceTime);
    p.elapsed = elapsed;
    p.speed = replaySpeed(sourceTime - p.cue.at, p.source[p.source.length - 1] - p.cue.at);
    return {
      snapshot,
      events: effects,
      time: sourceTime,
      dt,
      speed: p.speed,
      progress: Math.min(1, elapsed / duration),
      label: p.cue.label,
      focusSeat: p.cue.seat,
      focusPoint: p.cue.point ?? playerPoint(snapshot, p.cue.seat),
    };
  }

  /** A cloned historical room for presentation only; the actual room stays live. */
  sampleBoard(now = this.now()): BoardReplaySample | null { return this.board.sample(now); }

  private at(bout: Blender | null, key: string, at: number): GameSnap | null {
    const rows = this.frames;
    if (!rows.length) return null;
    let lo = 0, hi = rows.length - 1;
    while (hi - lo > 1) {
      const mid = (lo + hi) >> 1;
      if (rows[mid].at <= at) lo = mid; else hi = mid;
    }
    const a = rows[lo], b = rows[hi];
    if (keyOf(a.game) !== key || keyOf(b.game) !== key) return null;
    const t = Math.max(0, Math.min(1, (at - a.at) / Math.max(1e-7, b.at - a.at)));
    // Renderers may mutate a view; a replay can never alter retained history.
    return bout?.blend
      ? bout.blend(structuredClone(a.game), structuredClone(b.game), t) as GameSnap
      : structuredClone(t < 0.5 ? a.game : b.game);
  }
}

const BOARD_LIMITS = { history: 8, frames: 180, bytes: 4 * 1024 * 1024, queue: 20, post: 0.35, duration: 4 } as const;
type BoardFrame = { at: number; room: RoomView; bytes: number };
type BoardCue = { at: number; seat: number | null; point: Point | null; ready: number; expires: number; frames: BoardFrame[] | null };
type BoardPlan = { cue: BoardCue; source: number[]; wall: number[]; start: number; previous: number; elapsed: number; speed: number };

/** Board events are room log entries, so their presentation history is separate
 * from the minigame's snapshots. Queued attacker highlights never hide a turn. */
class BoardReplayDirector {
  private viewer = '';
  private identity = '';
  private frames: BoardFrame[] = [];
  private bytes = 0;
  private log: string[] | null = null;
  private safe = false;
  private cue: BoardCue | null = null;
  private plan: BoardPlan | null = null;
  private readonly now: () => number;
  constructor(now: () => number) { this.now = now; }

  get active() { return this.plan !== null; }
  // A deferred replay during the viewer's turn must never cover their controls.
  get pending() { return this.safe && this.cue !== null && !this.plan; }
  get status(): ReplayStatus | null {
    const p = this.plan;
    if (p) return { phase: 'playing', label: 'The knockout', speed: p.speed, progress: Math.min(1, p.elapsed / p.wall[p.wall.length - 1]) };
    return this.pending ? { phase: 'pending', label: 'Knockout replay', speed: 1, progress: 0 } : null;
  }
  setViewer(userId: string) { if (this.viewer !== userId) this.reset(); this.viewer = userId; }
  skip() { this.cue = null; this.plan = null; }
  reset() { this.skip(); this.frames = []; this.bytes = 0; this.log = null; this.identity = ''; this.safe = false; }

  observe(raw: Pick<RoomView, 'phase'> & Partial<RoomView>) {
    if (raw.phase.name !== 'board' || !raw.board || !raw.members || !raw.match_id) { this.reset(); return; }
    const now = this.now();
    const identity = `${raw.match_id}/${raw.board.map}`;
    if (this.identity !== identity) { this.reset(); this.identity = identity; }
    const own = raw.members.find((member) => member.user_id === this.viewer);
    this.safe = !own || own.seat !== raw.board.turn_seat;
    if (!this.safe && this.plan) this.skip();
    if (this.cue && now > this.cue.expires) this.skip();
    const room = raw as RoomView;
    const bytes = JSON.stringify(room).length * 2;
    if (bytes > BOARD_LIMITS.bytes) { this.reset(); return; }
    const frame = { at: now, room: structuredClone(room), bytes };
    this.frames.push(frame); this.bytes += bytes;
    while (this.frames.length > 1 && (this.frames.length > BOARD_LIMITS.frames || this.bytes > BOARD_LIMITS.bytes || this.frames[0].at < now - BOARD_LIMITS.history)) this.bytes -= this.frames.shift()!.bytes;

    const stamped = room.board!.log.map((entry) => JSON.stringify(entry));
    if (this.log !== null) {
      // Longest suffix/prefix overlap handles capped logs and identical repeated
      // entries, without reinterpreting a late join's old log as a new impact.
      let overlap = Math.min(this.log.length, stamped.length);
      while (overlap > 0 && !this.log.slice(-overlap).every((value, i) => value === stamped[i])) overlap--;
      for (const entry of room.board!.log.slice(overlap)) {
        if (entry.kind !== 'knockout' || this.plan || this.cue || now - this.frames[0].at < 0.35) continue;
        const seat = number(entry.seat) ? entry.seat : null;
        const token = room.board!.tokens.find((token) => token.seat === seat);
        const space = room.board!.spaces.find((space) => space.id === token?.space);
        this.cue = { at: now, seat, point: point(space), ready: now + BOARD_LIMITS.post, expires: now + BOARD_LIMITS.queue, frames: null };
      }
    }
    this.log = stamped;
    this.freeze(now);
  }

  private freeze(now: number) {
    const cue = this.cue;
    if (cue && !cue.frames && now >= cue.ready) {
      // Keep the actual pre-hit states while the attacker finishes their turn;
      // otherwise an eight-second rolling buffer could lose the whole impact.
      cue.frames = this.frames.filter((frame) => frame.at >= cue.at - 1.2 && frame.at <= cue.ready);
      const last = cue.frames[cue.frames.length - 1];
      // A board can become idle immediately after a knockout. Hold its actual
      // final state long enough for recorded effects and the exit ramp; this
      // adds no simulated movement or invented network state.
      if (last && last.at < cue.ready) cue.frames.push({ ...last, at: cue.ready });
      // Frozen and rolling views share immutable snapshots, not duplicate clones.
      // Each rolling/frozen segment has a 4 MiB limit (8 MiB combined).
      while (this.frames.length > 1 && this.frames[0].at < cue.ready) this.bytes -= this.frames.shift()!.bytes;
    }
  }

  sample(now: number): BoardReplaySample | null {
    if (!this.safe) return null;
    if (this.cue && now > this.cue.expires) this.skip();
    this.freeze(now);
    if (!this.plan && this.cue?.frames) {
      const cue = this.cue;
      const frames = cue.frames!;
      if (frames.length < 2 || cue.at - frames[0].at < 0.35) { this.skip(); return null; }
      const start = Math.max(frames[0].at, cue.at - 1.15), end = frames[frames.length - 1].at;
      const source = [start], wall = [0];
      for (let at = start; at < end - 1e-7;) {
        const next = Math.min(end, at + 1 / 240);
        wall.push(wall[wall.length - 1] + (next - at) / replaySpeed((at + next) / 2 - cue.at, end - cue.at));
        source.push(next); at = next;
      }
      if (wall.length < 2 || wall[wall.length - 1] > BOARD_LIMITS.duration) { this.skip(); return null; }
      this.plan = { cue, source, wall, start: now, previous: start, elapsed: 0, speed: 1 };
    }
    const p = this.plan;
    if (!p) return null;
    const elapsed = Math.max(0, now - p.start), duration = p.wall[p.wall.length - 1];
    if (elapsed > duration + 1e-6) { this.skip(); return null; }
    let lo = 0, hi = p.wall.length - 1;
    while (hi - lo > 1) { const mid = (lo + hi) >> 1; if (p.wall[mid] <= elapsed) lo = mid; else hi = mid; }
    const t = Math.max(0, Math.min(1, (elapsed - p.wall[lo]) / Math.max(1e-7, p.wall[hi] - p.wall[lo])));
    const time = p.source[lo] + (p.source[hi] - p.source[lo]) * t;
    const frames = p.cue.frames!;
    let index = 0;
    while (index + 1 < frames.length && frames[index + 1].at <= time + 1e-7) index++;
    const room = structuredClone(frames[index].room);
    const next = frames[index + 1];
    const a = room.board?.walk, b = next?.room.board?.walk;
    if (a && b && a.id === b.id) {
      const f = Math.max(0, Math.min(1, (time - frames[index].at) / Math.max(1e-7, next.at - frames[index].at)));
      a.x += (b.x - a.x) * f; a.z += (b.z - a.z) * f; a.elapsed += (b.elapsed - a.elapsed) * f;
    }
    const dt = Math.max(0, Math.min(0.1, time - p.previous));
    p.previous = Math.max(p.previous, time); p.elapsed = elapsed;
    p.speed = replaySpeed(time - p.cue.at, p.source[p.source.length - 1] - p.cue.at);
    return { room, time, dt, speed: p.speed, progress: Math.min(1, elapsed / duration), label: 'The knockout', focusSeat: p.cue.seat, focusPoint: p.cue.point };
  }
}
