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
  type MicrophoneState,
  type PitchSample,
  type Presence,
} from './protocol';

export type { Json, Lobby, Me, MicrophoneState, PitchSample, Presence } from './protocol';

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
  /**
   * Starts or stops pitch from the player's microphone, for singing games.
   * tawks asks the player before turning it on and only ever sends pitch,
   * never audio. Resolves with the resulting state.
   */
  microphone(on: boolean): Promise<MicrophoneState>;
  /** Calls `listener` with each batch of pitch samples; returns a function that stops it. */
  onPitch(listener: (samples: PitchSample[]) => void): () => void;
  /**
   * Puts the game full screen, or leaves it. A frame is not allowed to do
   * this for itself, so tawks takes the panel around it full screen instead.
   * Resolves with the state it ended up in.
   */
  fullscreen(on: boolean): Promise<boolean>;
  /** Calls `listener` whenever full screen goes on or off; returns a function that stops it. */
  onFullscreen(listener: (on: boolean) => void): () => void;
  /**
   * Says what the player is doing, such as the song and part, for their
   * status and profile card. tawks shows it only to the people they share
   * what they play with, and clears it when the game closes. Pass null to clear it.
   */
  setPresence(presence: Partial<Presence> | null): void;
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
  hideNativeContextMenu();

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
  const pitchListeners = new Set<(samples: PitchSample[]) => void>();
  let microphoneWaiters: ((state: MicrophoneState) => void)[] = [];
  let fullscreenWaiters: ((on: boolean) => void)[] = [];
  const pending = new Map<number, { resolve: () => void; reject: (error: Error) => void }>();

  const askFullscreen = (on: boolean) =>
    new Promise<boolean>((resolve) => {
      // A tawks page from before full screen was in the bridge never answers,
      // so give up rather than leave the game's button stuck.
      const waiter = (state: boolean) => {
        clearTimeout(timer);
        resolve(state);
      };
      const timer = setTimeout(() => {
        fullscreenWaiters = fullscreenWaiters.filter((pending) => pending !== waiter);
        resolve(false);
      }, 1000);
      fullscreenWaiters.push(waiter);
      port.postMessage({ type: 'fullscreen', on } satisfies GameMessage);
    });
  askHost = askFullscreen;

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
          microphone(on) {
            return new Promise((resolve) => {
              microphoneWaiters.push(resolve);
              port.postMessage({ type: 'microphone', on } satisfies GameMessage);
            });
          },
          onPitch(listener) {
            pitchListeners.add(listener);
            return () => pitchListeners.delete(listener);
          },
          fullscreen: askFullscreen,
          setPresence(presence) {
            const full = presence && { details: presence.details ?? null, state: presence.state ?? null, started_at: presence.started_at ?? null };
            port.postMessage({ type: 'presence', presence: full } satisfies GameMessage);
          },
          onFullscreen(listener) {
            fullscreenListeners.add(listener);
            return () => fullscreenListeners.delete(listener);
          },
        });
        break;
      case 'state':
        state = message.state as State;
        for (const listener of listeners) listener(state);
        break;
      case 'microphone': {
        const waiting = microphoneWaiters;
        microphoneWaiters = [];
        for (const resolve of waiting) resolve(message.state);
        break;
      }
      case 'fullscreen': {
        const waiting = fullscreenWaiters;
        fullscreenWaiters = [];
        for (const resolve of waiting) resolve(message.on);
        for (const listener of fullscreenListeners) listener(message.on);
        break;
      }
      case 'pitch':
        for (const listener of pitchListeners) listener(message.samples);
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

let contextMenuHidden = false;

/**
 * Hides the browser's own right-click menu over the game, which tawks cannot
 * do from outside the frame. Fields and selected text keep it.
 */
function hideNativeContextMenu() {
  if (contextMenuHidden) return;
  contextMenuHidden = true;
  window.addEventListener('contextmenu', (event) => {
    const element = event.target instanceof Element ? event.target : null;
    if (!element || element.closest('input,textarea,select')) return;
    if (element instanceof HTMLElement && element.isContentEditable) return;
    const selection = window.getSelection();
    if (selection && !selection.isCollapsed && selection.containsNode(element, true)) return;
    event.preventDefault();
  });
}

/** Set once a frame is connected, so `goFullscreen` can ask tawks. */
let askHost: ((on: boolean) => Promise<boolean>) | null = null;
const fullscreenListeners = new Set<(on: boolean) => void>();

/**
 * Puts `element` full screen, or leaves full screen. Inside an activity frame
 * the request goes to tawks, which takes the panel around the frame, since a
 * frame is not allowed to do it itself. Returns the state it ended up in.
 */
export async function goFullscreen(on: boolean, element: Element): Promise<boolean> {
  if (askHost) return askHost(on);
  if (!on) {
    if (document.fullscreenElement) await document.exitFullscreen();
    return false;
  }
  await element.requestFullscreen();
  return true;
}

/**
 * Calls `listener` whenever full screen goes on or off, framed or not, so a
 * game's own button keeps up with Escape and with the player's other ways out.
 */
export function onFullscreenChange(listener: (on: boolean) => void): () => void {
  const onChange = () => listener(!!document.fullscreenElement);
  fullscreenListeners.add(listener);
  document.addEventListener('fullscreenchange', onChange);
  return () => {
    fullscreenListeners.delete(listener);
    document.removeEventListener('fullscreenchange', onChange);
  };
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
