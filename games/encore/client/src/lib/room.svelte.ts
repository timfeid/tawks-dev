import type { ClientMsg, Pass, RoomView, ServerMsg } from './types';

type Sample = { rtt: number; offset: number };

/**
 * The lobby's room socket. Besides the room itself it keeps an estimate of
 * the server clock, so every player can start the song at the same moment.
 */
export class RoomConnection {
  status = $state<'connecting' | 'online' | 'offline'>('connecting');
  room = $state<RoomView | null>(null);
  conn = $state<number | null>(null);
  userId = $state<string | null>(null);
  settings = $state<unknown>(null);
  bests = $state<Record<string, number>>({});
  error = $state<string | null>(null);
  library = $state(false);
  uploads = $state(false);
  rtt = $state<number | null>(null);

  private socket: WebSocket | null = null;
  private samples: Sample[] = [];
  private offset = 0;
  private pings = 0;
  private pingTimer: ReturnType<typeof setTimeout> | null = null;
  private retry = 0;
  private closed = false;
  private errorTimer: ReturnType<typeof setTimeout> | null = null;
  private cueListeners = new Set<(conn: number, t: number, out: boolean) => void>();

  constructor(
    private pass: Pass,
    private renewPass: () => Promise<Pass>,
  ) {
    this.open();
  }

  get current() {
    return this.pass;
  }

  /** After a reconnect the room may still list the old connection, so fall back to the account. */
  get me() {
    const players = this.room?.players ?? [];
    return players.find((p) => p.conn === this.conn) ?? players.find((p) => p.user_id === this.userId) ?? null;
  }

  /** The server's clock, in Unix milliseconds. */
  serverNow() {
    return performance.now() + this.offset;
  }

  /** Shows a message the way server errors show. */
  flash(message: string) {
    this.error = message;
    if (this.errorTimer) clearTimeout(this.errorTimer);
    this.errorTimer = setTimeout(() => (this.error = null), 4_000);
  }

  send(message: ClientMsg) {
    if (this.socket?.readyState === WebSocket.OPEN) this.socket.send(JSON.stringify(message));
  }

  /** Hears other players drop out of and come back into the song. */
  onCue(listener: (conn: number, t: number, out: boolean) => void) {
    this.cueListeners.add(listener);
    return () => this.cueListeners.delete(listener);
  }

  saveSettings(settings: unknown) {
    this.settings = settings;
    this.send({ type: 'settings', settings });
  }

  close() {
    this.closed = true;
    if (this.pingTimer) clearTimeout(this.pingTimer);
    this.socket?.close();
  }

  private open() {
    if (this.closed) return;
    this.status = 'connecting';
    const socket = new WebSocket(this.pass.ws_url);
    this.socket = socket;
    socket.onopen = () => {
      this.retry = 0;
      this.pings = 0;
      socket.send(JSON.stringify({ type: 'hello', token: this.pass.token } satisfies ClientMsg));
      this.ping();
    };
    socket.onmessage = (event) => this.receive(JSON.parse(event.data) as ServerMsg);
    socket.onclose = () => {
      if (this.socket !== socket) return;
      this.status = 'offline';
      if (this.pingTimer) clearTimeout(this.pingTimer);
      if (this.closed) return;
      const delay = Math.min(10_000, 500 * 2 ** this.retry++);
      setTimeout(() => this.open(), delay);
    };
  }

  private ping() {
    this.send({ type: 'ping', t: performance.now() });
    // Many quick samples at first so the clock settles before anyone presses ready.
    const every = this.pings++ < 12 ? 250 : 2_000;
    this.pingTimer = setTimeout(() => this.ping(), every);
  }

  private receive(message: ServerMsg) {
    switch (message.type) {
      case 'welcome':
        this.conn = message.conn;
        this.userId = message.user_id;
        this.settings = message.settings;
        this.bests = message.bests;
        this.library = !!message.library;
        this.uploads = !!message.uploads;
        this.status = 'online';
        break;
      case 'room':
        this.room = message.room;
        break;
      case 'pong': {
        const now = performance.now();
        const rtt = now - message.t;
        this.samples.push({ rtt, offset: message.now + rtt / 2 - now });
        if (this.samples.length > 20) this.samples.shift();
        const best = this.samples.reduce((a, b) => (b.rtt < a.rtt ? b : a));
        this.offset = best.offset;
        this.rtt = Math.round(best.rtt);
        break;
      }
      case 'cue':
        for (const listener of this.cueListeners) listener(message.conn, message.t, message.out);
        break;
      case 'error':
        if (message.message === 'pass_expired') {
          this.renewPass()
            .then((pass) => {
              this.pass = pass;
              this.socket?.close();
            })
            .catch(() => (this.error = 'Your pass expired. Reopen the game.'));
          return;
        }
        this.flash(message.message);
        break;
    }
  }
}
