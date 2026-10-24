import {
  PROTOCOL,
  VERSION,
  isWelcome,
  mouseBinding,
  type GameMessage,
  type HostMessage,
  type Json,
  type Lobby,
  type Me,
} from './protocol';

export type { Json, Lobby, Me } from './protocol';

export type Activity<State = Json> = {
  /** The player this frame belongs to. */
  me: Me;
  /** The lobby the game runs in, or null on a channel page. */
  lobby: Lobby | null;
  /** The latest state your bot sent this player. */
  readonly state: State;
  /** Calls `listener` with every new state; returns a function that stops it. */
  onState(listener: (state: State) => void): () => void;
  /** Sends a move to your bot. Resolves once tawks delivers it; your bot answers with new state. */
  send(input: Json): Promise<void>;
  /**
   * Runs one of your workflow's own actions with these inputs, for games that
   * use several actions instead of one input action. Keys starting with `$`
   * are reserved.
   */
  call(action: string, inputs: { [key: string]: Json }): Promise<void>;
};

export type ConnectOptions = {
  /** How long to wait for tawks before giving up. Defaults to 10 seconds. */
  timeoutMs?: number;
};

/** Connects to the tawks page this game is framed in. */
export function connect<State = Json>({ timeoutMs = 10_000 }: ConnectOptions = {}): Promise<Activity<State>> {
  if (window.parent === window) {
    return Promise.reject(new Error('Not running inside a tawks activity frame'));
  }

  return new Promise((resolve, reject) => {
    const hello = { protocol: PROTOCOL, version: VERSION, type: 'hello' };
    const say = () => window.parent.postMessage(hello, '*');
    const retry = setInterval(say, 500);
    const timeout = setTimeout(() => {
      cleanup();
      reject(new Error('tawks did not answer'));
    }, timeoutMs);

    function cleanup() {
      clearInterval(retry);
      clearTimeout(timeout);
      window.removeEventListener('message', onWelcome);
    }

    function onWelcome(event: MessageEvent) {
      if (event.source !== window.parent || !isWelcome(event.data) || !event.ports[0]) return;
      cleanup();
      attach(event.ports[0], resolve);
    }

    window.addEventListener('message', onWelcome);
    say();
  });
}

function attach<State>(port: MessagePort, resolve: (activity: Activity<State>) => void) {
  let state: State;
  let nextId = 1;
  const listeners = new Set<(state: State) => void>();
  const pending = new Map<number, { resolve: () => void; reject: (error: Error) => void }>();

  function request(message: (id: number) => GameMessage) {
    const id = nextId++;
    return new Promise<void>((resolve, reject) => {
      pending.set(id, { resolve, reject });
      port.postMessage(message(id));
    });
  }

  port.onmessage = (event: MessageEvent<HostMessage>) => {
    const message = event.data;
    switch (message.type) {
      case 'init':
        state = message.state as State;
        resolve({
          me: message.me,
          lobby: message.lobby,
          get state() {
            return state;
          },
          onState(listener) {
            listeners.add(listener);
            return () => listeners.delete(listener);
          },
          send(input) {
            return request((id) => ({ type: 'send', id, input }));
          },
          call(action, inputs) {
            return request((id) => ({ type: 'call', id, action, inputs }));
          },
        });
        break;
      case 'state':
        state = message.state as State;
        for (const listener of listeners) listener(state);
        break;
      case 'push_to_talk':
        pushToTalk.watch(message.binding, (down) => port.postMessage({ type: 'push_to_talk', down }));
        break;
      case 'result': {
        const waiting = pending.get(message.id);
        pending.delete(message.id);
        if (!waiting) break;
        if (message.ok) waiting.resolve();
        else waiting.reject(new Error(message.error));
        break;
      }
    }
  };
  port.start();
}

/**
 * Reports the player's push-to-talk key or button to tawks, since presses
 * inside the frame never reach it. Only that one binding is reported.
 */
const pushToTalk = {
  binding: null as string | null,
  report: null as ((down: boolean) => void) | null,
  down: false,
  installed: false,

  watch(binding: string | null, report: (down: boolean) => void) {
    if (this.down && binding !== this.binding) this.set(false);
    this.binding = binding;
    this.report = report;
    if (this.installed) return;
    this.installed = true;
    const opts = { capture: true };
    window.addEventListener('keydown', (e) => !e.repeat && this.match(e.key.toLowerCase()) && this.set(true), opts);
    window.addEventListener('keyup', (e) => this.match(e.key.toLowerCase()) && this.set(false), opts);
    window.addEventListener('mousedown', (e) => this.match(mouseBinding(e.button)) && this.set(true), opts);
    window.addEventListener('mouseup', (e) => this.match(mouseBinding(e.button)) && this.set(false), opts);
    // A release while the frame lost focus never arrives here.
    window.addEventListener('blur', () => this.set(false));
  },

  match(binding: string) {
    return this.binding !== null && binding === this.binding;
  },

  set(down: boolean) {
    if (down === this.down) return;
    this.down = down;
    this.report?.(down);
  },
};
