import type { ClientMsg, FrameDelta, GameMeta, Input, RelayMsg, RosterSeat, SceneFrame } from './contract';
import type { MadeSource, SourceStatus } from './connection';
import { diffFrames } from './scene-buffer';

/** The parts of the kit's `createRunner` a local room drives. */
export type LocalRunner = {
  meta: GameMeta;
  input(seat: number, input: Input): void;
  step(): number[][] | null;
  frame(): SceneFrame;
  readonly time: number;
};

const TICK_HZ = 60;
const FRAME_EVERY = 3;
const KEYFRAME_EVERY = 40;

/**
 * Plays a round in the page, standing in for the relay: welcome, frames at
 * 20 a second as deltas, the end, and a restart. For previews and tests of
 * the renderer only; real games run on the server.
 */
export class LocalSource implements MadeSource {
  private handlers = new Set<(message: RelayMsg) => void>();
  private timer: ReturnType<typeof setInterval> | null = null;
  private runner: LocalRunner;
  private last: SceneFrame | null = null;
  private ticks = 0;
  private frames = 0;
  private over = false;

  constructor(
    private readonly start: () => LocalRunner,
    private readonly roster: RosterSeat[],
    private readonly seat: number | null,
  ) {
    this.runner = start();
    queueMicrotask(() => this.begin());
  }

  private emit(message: RelayMsg) {
    for (const handler of this.handlers) handler(message);
  }

  private begin() {
    this.emit({
      type: 'welcome',
      welcome: { seat: this.seat, user_id: 'local', owner: true, lobby_id: 'local', activity_id: 'local', version: 0, meta: this.runner.meta, roster: this.roster },
    });
    this.timer = setInterval(() => this.tick(), 1000 / TICK_HZ);
  }

  private tick() {
    if (this.over) return;
    const placements = this.runner.step();
    this.ticks++;
    if (this.ticks % FRAME_EVERY === 0 || placements) {
      const frame = this.runner.frame();
      const whole = !this.last || this.frames % KEYFRAME_EVERY === 0;
      const delta: FrameDelta = diffFrames(whole ? null : this.last, frame);
      this.last = frame;
      this.frames++;
      this.emit({ type: 'frame', frame: delta });
    }
    if (placements) {
      this.over = true;
      this.emit({ type: 'end', placements, reason: 'finished' });
    }
  }

  restart() {
    this.runner = this.start();
    this.last = null;
    this.ticks = 0;
    this.frames = 0;
    this.over = false;
    this.emit({ type: 'restarting', version: 0 });
  }

  subscribe(handler: (message: RelayMsg) => void) {
    this.handlers.add(handler);
    return () => void this.handlers.delete(handler);
  }

  onStatus(handler: (status: SourceStatus) => void) {
    handler('online');
    return () => {};
  }

  send(message: ClientMsg) {
    if (message.type === 'input' && this.seat !== null) this.runner.input(this.seat, message.input);
    if (message.type === 'resync') this.last = null;
    if (message.type === 'restart') this.restart();
    return true;
  }

  close() {
    if (this.timer) clearInterval(this.timer);
    this.timer = null;
  }
}
