import type { BoutModule } from '@tawks/bout-sdk';
import type { ClientMsg, GameEvent, GameSnap, RoomView, ServerMsg, CharacterDef } from './protocol';

export type ConnectionStatus = 'connecting' | 'online' | 'offline';

export type PartyHandlers = {
  welcome: (userId: string, roster: CharacterDef[], guest: boolean) => void;
  room: (room: RoomView, spectating: boolean) => void;
  events: (events: GameEvent[]) => void;
  /** The card table as this seat may see it, by game key. */
  table: (game: string, view: unknown) => void;
  error: (message: string) => void;
  status: (status: ConnectionStatus, rttMs: number | null) => void;
  /** The pass was rejected; fetch a new one and call `setPass`. */
  passExpired: () => void;
};

type Timed = { serverTime: number; game: GameSnap };

const INTERPOLATION_DELAY = 0.1;

/**
 * One WebSocket to a party room. Keeps a short buffer of snapshots and
 * reports the arena as it looked a moment ago, so motion stays smooth
 * between the server's 20 Hz updates.
 */
export class PartyConnection {
  private socket: WebSocket | null = null;
  private url = '';
  private token = '';
  private closed = false;
  private retry = 0;
  private retryTimer: ReturnType<typeof setTimeout> | null = null;
  private pingTimer: ReturnType<typeof setInterval> | null = null;
  private buffer: Timed[] = [];
  private tickHz = 60;
  private clockOffset: number | null = null;
  rtt: number | null = null;

  constructor(private handlers: PartyHandlers) {}

  setPass(url: string, token: string) {
    const changed = url !== this.url || token !== this.token;
    this.url = url;
    this.token = token;
    if (changed || !this.socket) this.open();
  }

  send(msg: ClientMsg) {
    if (this.socket?.readyState === WebSocket.OPEN) this.socket.send(JSON.stringify(msg));
  }

  close() {
    this.closed = true;
    if (this.retryTimer) clearTimeout(this.retryTimer);
    if (this.pingTimer) clearInterval(this.pingTimer);
    this.socket?.close();
    this.socket = null;
  }

  private open() {
    if (this.closed || !this.url || !this.token) return;
    if (this.retryTimer) clearTimeout(this.retryTimer);
    const previous = this.socket;
    this.socket = null;
    previous?.close();
    this.handlers.status('connecting', null);
    const socket = new WebSocket(this.url);
    this.socket = socket;
    let expired = false;
    socket.onopen = () => {
      socket.send(JSON.stringify({ type: 'hello', token: this.token } satisfies ClientMsg));
      this.retry = 0;
      if (this.pingTimer) clearInterval(this.pingTimer);
      this.pingTimer = setInterval(() => this.send({ type: 'ping', t: performance.now() }), 2000);
    };
    socket.onmessage = (event) => {
      let msg: ServerMsg;
      try {
        msg = JSON.parse(event.data);
      } catch {
        return;
      }
      switch (msg.type) {
        case 'welcome':
          this.tickHz = msg.tick_hz;
          this.buffer = [];
          this.clockOffset = null;
          this.handlers.welcome(msg.user_id, msg.roster, msg.guest === true);
          this.handlers.status('online', this.rtt);
          break;
        case 'room':
          this.handlers.room(msg.room, msg.spectating);
          break;
        case 'snap':
          this.receive(msg.tick, msg.game);
          if (msg.events.length) this.handlers.events(msg.events);
          break;
        case 'table':
          this.handlers.table(msg.game, msg.view);
          break;
        case 'pong':
          this.rtt = performance.now() - msg.t;
          this.handlers.status('online', this.rtt);
          break;
        case 'error':
          if (msg.code === 'auth') {
            expired = true;
            this.handlers.passExpired();
          } else {
            this.handlers.error(msg.message);
          }
          break;
      }
    };
    socket.onclose = () => {
      if (this.socket !== socket) return;
      this.socket = null;
      if (this.pingTimer) clearInterval(this.pingTimer);
      this.handlers.status('offline', null);
      if (this.closed || expired) return;
      const wait = Math.min(8000, 500 * 2 ** this.retry++);
      this.retryTimer = setTimeout(() => this.open(), wait);
    };
  }

  private receive(tick: number, game: GameSnap) {
    const serverTime = tick / this.tickHz;
    const now = performance.now() / 1000;
    const offset = now - serverTime;
    // Track the quickest arrival; late packets should not drag the clock.
    this.clockOffset = this.clockOffset === null || offset < this.clockOffset ? offset : this.clockOffset + (offset - this.clockOffset) * 0.02;
    if (this.buffer.length && serverTime < this.buffer[this.buffer.length - 1].serverTime) {
      this.buffer = [];
    }
    this.buffer.push({ serverTime, game });
    if (this.buffer.length > 30) this.buffer.splice(0, this.buffer.length - 30);
  }

  /** The arena as of a moment ago, blended between the two nearest snapshots by the bout's own rules. */
  sample(bout: Pick<BoutModule, 'blend' | 'extrapolate'> | null): GameSnap | null {
    if (!this.buffer.length || this.clockOffset === null) return null;
    const renderTime = performance.now() / 1000 - this.clockOffset - INTERPOLATION_DELAY;
    const newest = this.buffer[this.buffer.length - 1];
    if (this.buffer.length === 1 || renderTime >= newest.serverTime) {
      const late = Math.min(renderTime - newest.serverTime, 0.1);
      return late > 0 && bout?.extrapolate ? (bout.extrapolate(newest.game, late) as GameSnap) : newest.game;
    }
    let i = this.buffer.length - 2;
    while (i > 0 && this.buffer[i].serverTime > renderTime) i--;
    const a = this.buffer[i];
    const b = this.buffer[i + 1];
    const t = Math.max(0, Math.min(1, (renderTime - a.serverTime) / Math.max(1e-6, b.serverTime - a.serverTime)));
    if (!bout?.blend || a.game.game !== b.game.game) return b.game;
    return bout.blend(a.game, b.game, t) as GameSnap;
  }
}
