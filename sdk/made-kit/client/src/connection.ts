import type { ClientMsg, MadeJoin, RelayMsg } from './contract';

export type SourceStatus = 'connecting' | 'online' | 'offline';

/** Where scene frames come from: the relay, or a local stand-in in tests and previews. */
export interface MadeSource {
  subscribe(handler: (message: RelayMsg) => void): () => void;
  onStatus(handler: (status: SourceStatus) => void): () => void;
  /** False when the message was not sent, so the caller tries again next frame. */
  send(message: ClientMsg): boolean;
  close(): void;
}

export type MadePass = Pick<MadeJoin, 'ws_url' | 'token'>;

/** The relay takes at most 60 inputs a second and drops the rest. */
const MIN_INPUT_GAP_MS = 1000 / 60;

/**
 * The relay socket. It says hello with the pass, reconnects with backoff, and
 * asks `renew` for a fresh pass when the relay closes with a fatal error.
 */
export class MadeSocket implements MadeSource {
  private socket: WebSocket | null = null;
  private handlers = new Set<(message: RelayMsg) => void>();
  private statusHandlers = new Set<(status: SourceStatus) => void>();
  private retry = 0;
  private closed = false;
  private lastInput = 0;
  private status: SourceStatus = 'connecting';
  private needsPass = false;

  constructor(
    private pass: MadePass,
    private readonly renew?: () => Promise<MadePass | null>,
  ) {
    this.open();
  }

  subscribe(handler: (message: RelayMsg) => void) {
    this.handlers.add(handler);
    return () => void this.handlers.delete(handler);
  }

  onStatus(handler: (status: SourceStatus) => void) {
    this.statusHandlers.add(handler);
    handler(this.status);
    return () => void this.statusHandlers.delete(handler);
  }

  private setStatus(status: SourceStatus) {
    this.status = status;
    for (const handler of this.statusHandlers) handler(status);
  }

  send(message: ClientMsg) {
    if (this.socket?.readyState !== WebSocket.OPEN) return false;
    if (message.type === 'input') {
      const now = performance.now();
      if (now - this.lastInput < MIN_INPUT_GAP_MS) return false;
      this.lastInput = now;
    }
    this.socket.send(JSON.stringify(message));
    return true;
  }

  close() {
    this.closed = true;
    this.socket?.close();
    this.socket = null;
  }

  private async open() {
    if (this.closed) return;
    this.setStatus('connecting');
    if (this.needsPass && this.renew) {
      const pass = await this.renew().catch(() => null);
      if (pass) this.pass = pass;
      this.needsPass = false;
      if (this.closed) return;
    }
    const socket = new WebSocket(this.pass.ws_url);
    this.socket = socket;
    socket.onopen = () => {
      this.retry = 0;
      socket.send(JSON.stringify({ type: 'hello', token: this.pass.token } satisfies ClientMsg));
    };
    socket.onmessage = (event) => {
      let message: RelayMsg;
      try {
        message = JSON.parse(event.data as string) as RelayMsg;
      } catch {
        return;
      }
      if (message.type === 'welcome' || message.type === 'sim_welcome') this.setStatus('online');
      if (message.type === 'error' && message.fatal) this.needsPass = true;
      for (const handler of this.handlers) handler(message);
    };
    socket.onclose = () => {
      if (this.socket !== socket) return;
      this.setStatus('offline');
      if (this.closed) return;
      const delay = Math.min(10_000, 400 * 2 ** this.retry++);
      setTimeout(() => void this.open(), delay);
    };
  }
}
