import type { Action, ClientMsg, Condition, Pass, ServerMsg, TableView } from './types';

/** The lobby's table socket. The server's table is the only truth; this keeps the latest copy. */
export class TableConnection {
  status = $state<'connecting' | 'online' | 'offline'>('connecting');
  table = $state<TableView | null>(null);
  conditions = $state<Condition[]>([]);
  attribution = $state('');
  compatibility = $state('');
  error = $state<string | null>(null);
  /** Rolls that just landed, for the tray to animate. */
  fresh = $state<number[]>([]);
  freshAt = $state(0);

  private socket: WebSocket | null = null;
  private retry = 0;
  private closed = false;
  private pingTimer: ReturnType<typeof setInterval> | null = null;
  private errorTimer: ReturnType<typeof setTimeout> | null = null;

  constructor(
    private pass: Pass,
    private renewPass: () => Promise<Pass>,
  ) {
    this.open();
  }

  get me() {
    return this.table?.me ?? null;
  }

  get isGm() {
    return !!this.table && this.table.gm === this.table.me;
  }

  act(action: Action) {
    if (this.socket?.readyState !== WebSocket.OPEN) {
      this.flash('Reconnecting to the table...');
      return;
    }
    this.socket.send(JSON.stringify({ type: 'act', action } satisfies ClientMsg));
  }

  flash(message: string) {
    this.error = message;
    if (this.errorTimer) clearTimeout(this.errorTimer);
    this.errorTimer = setTimeout(() => (this.error = null), 4_000);
  }

  close() {
    this.closed = true;
    if (this.pingTimer) clearInterval(this.pingTimer);
    this.socket?.close();
  }

  private open() {
    if (this.closed) return;
    this.status = 'connecting';
    const socket = new WebSocket(this.pass.ws_url);
    this.socket = socket;
    socket.onopen = () => {
      this.retry = 0;
      socket.send(JSON.stringify({ type: 'hello', token: this.pass.token } satisfies ClientMsg));
      if (this.pingTimer) clearInterval(this.pingTimer);
      this.pingTimer = setInterval(() => {
        if (socket.readyState === WebSocket.OPEN) socket.send(JSON.stringify({ type: 'ping', t: Date.now() }));
      }, 20_000);
    };
    socket.onmessage = (event) => this.receive(JSON.parse(event.data) as ServerMsg);
    socket.onclose = () => {
      if (this.socket !== socket) return;
      this.status = 'offline';
      if (this.closed) return;
      const delay = Math.min(10_000, 400 * 2 ** this.retry++);
      setTimeout(() => this.open(), delay);
    };
  }

  private receive(message: ServerMsg) {
    switch (message.type) {
      case 'welcome':
        this.conditions = message.conditions;
        this.attribution = message.attribution;
        this.compatibility = message.compatibility;
        this.status = 'online';
        break;
      case 'table':
        this.table = message.table;
        if (message.rolled.length) {
          this.fresh = message.rolled;
          this.freshAt = Date.now();
        }
        break;
      case 'error':
        if (message.message === 'pass_expired') {
          this.renewPass()
            .then((pass) => {
              this.pass = pass;
              this.socket?.close();
            })
            .catch(() => this.flash('Your seat expired. Reopen the table.'));
          return;
        }
        this.flash(message.message);
        break;
    }
  }
}
