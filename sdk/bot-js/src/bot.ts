import type {
  CreateWorkflowDefinition,
  JsonValue,
  ServerActionResult,
  UserResource,
  WorkflowResourceWithContext,
} from '@tawks/workflow-types';
import { TawksApi, requireTls, type ApiOptions } from './api.js';

export type BotOptions = ApiOptions & {
  /** For example `wss://events.tawks.app/`, or `ws://localhost:8787/events` for devhost. */
  eventsUrl: string;
  /** Where the bot reports what it is doing. Defaults to the console; pass `null` for silence. */
  logger?: Logger | null;
};

export type Logger = Pick<Console, 'info' | 'warn' | 'error'>;

/** Anything tawks sends the bot other than the server action requests it handles itself. */
export type BotMessage = { type: string; [key: string]: unknown };

/** A message posted in a chat the bot can see. */
export type ChatMessage = {
  id: string;
  chat_id: string;
  content: string;
  user: UserResource;
  parent_message_id?: string | null;
  timestamp?: string;
};

export type ServerActionHandler = (ctx: ServerActionContext) => ServerActionResult | Promise<ServerActionResult>;

export type Activity = {
  workflow_id: string;
  name: string;
  client_url: string;
  description?: string;
  icon?: string;
  min_players?: number;
  max_players?: number;
};

type Events = {
  connect: () => void;
  disconnect: (reason: string) => void;
  message: (message: BotMessage) => void;
  error: (error: Error) => void;
};

const HEARTBEAT_MS = 25_000;
const RENEW_MS = 15_000;
const DEDUPE_MS = 10 * 60_000;
const MAX_BACKOFF_MS = 30_000;

/** What a server action handler receives: the viewer's instance and its context. */
export class ServerActionContext {
  constructor(readonly request: WorkflowResourceWithContext) {}

  get workflow() {
    return this.request.workflow;
  }

  /** The instance's responses, including the inputs the page just sent. */
  get responses(): Partial<Record<string, JsonValue>> {
    return this.request.workflow.responses;
  }

  get context(): Partial<Record<string, JsonValue>> {
    return this.request.context;
  }

  /** The person using the page. */
  get user(): UserResource {
    return this.request.context.$user as unknown as UserResource;
  }

  /** True for a visitor who has not signed up. */
  get anonymous(): boolean {
    return this.request.context.$anonymous === true;
  }

  input<T = JsonValue>(key: string): T | undefined {
    return this.responses[key] as T | undefined;
  }

  /** Inputs from templates arrive as strings, so this accepts `"7"` as well as `7`. */
  number(key: string): number | undefined {
    const value = this.responses[key];
    const n = typeof value === 'string' && value.trim() !== '' ? Number(value) : value;
    return typeof n === 'number' && Number.isFinite(n) ? n : undefined;
  }
}

export const result = {
  none: (): ServerActionResult => 'None',
  error: (message: string): ServerActionResult => ({ Error: message }),
  responses: (responses: Partial<Record<string, JsonValue>>): ServerActionResult => ({ UpdateResponses: responses }),
  context: (context: Partial<Record<string, JsonValue>>): ServerActionResult => ({ UpdateContext: context }),
  complete: (message: string, responses: Partial<Record<string, JsonValue>> = {}): ServerActionResult =>
    ({ CompleteWorkflow: { message, responses, close_active_modal: false } }),
};

/**
 * A tawks bot. Register server actions and workflows, then call `run()`. It
 * keeps the events socket open, renews server action leases, and registers
 * everything again when tawks restarts or the connection drops.
 */
export class TawksBot {
  readonly api: TawksApi;
  readonly registrarId = `js-bot-${crypto.randomUUID()}`;
  private readonly eventsUrl: string;
  private readonly log: Logger | null;
  private readonly handlers = new Map<string, ServerActionHandler>();
  private readonly actionIds = new Map<string, string>();
  private readonly workflows = new Map<string, CreateWorkflowDefinition>();
  private readonly activities = new Map<string, Activity>();
  private readonly listeners: { [K in keyof Events]: Set<Events[K]> } = {
    connect: new Set(),
    disconnect: new Set(),
    message: new Set(),
    error: new Set(),
  };
  private readonly seenTokens = new Map<string, number>();
  private socket: WebSocket | null = null;
  private stopped = false;
  private wake: (() => void) | null = null;

  constructor(options: BotOptions) {
    requireTls(options.eventsUrl);
    this.api = new TawksApi(options);
    this.eventsUrl = options.eventsUrl;
    this.log = options.logger === undefined ? console : options.logger;
  }

  on<K extends keyof Events>(event: K, listener: Events[K]): () => void {
    this.listeners[event].add(listener);
    return () => this.listeners[event].delete(listener);
  }

  /** Calls `listener` for each chat message, skipping messages from bots, this one included. */
  onChat(listener: (message: ChatMessage) => void | Promise<void>): () => void {
    return this.on('message', (event) => {
      if (event.type !== 'chat_message') return;
      const message = event.message as ChatMessage;
      if (message.user.is_bot) return;
      Promise.resolve(listener(message)).catch((error) =>
        this.emit('error', error instanceof Error ? error : new Error(String(error))),
      );
    });
  }

  /** Posts a message in a chat. For a chat channel, the chat id is the channel id. */
  say(chatId: string, text: string): Promise<unknown> {
    return this.api.mutate('chat_create_message', { chat_id: chatId, contents: { text, attachments: null } });
  }

  /** Registers a server action and returns its full id, which workflow actions target. */
  async serverAction(actionId: string, handler: ServerActionHandler): Promise<string> {
    const id = await this.api.mutate<string>('server_action_register', {
      action_id: actionId,
      registrar_id: this.registrarId,
    });
    this.handlers.set(id, handler);
    this.actionIds.set(actionId, id);
    return id;
  }

  /** Registers a workflow definition and returns its full id, which a channel's settings list. */
  async workflow(definition: CreateWorkflowDefinition): Promise<string> {
    const id = await this.api.mutate<string>('workflow_register_workflow', definition);
    this.workflows.set(id, definition);
    return id;
  }

  /** Registers a game activity for lobbies' game menus. */
  async activity(activity: Activity): Promise<void> {
    await this.api.mutate('activity_register', activity);
    this.activities.set(activity.workflow_id, activity);
  }

  /** Connects and stays connected until `stop()`. Resolves once stopped. */
  async run(): Promise<void> {
    let attempt = 0;
    let first = true;
    while (!this.stopped) {
      const started = Date.now();
      try {
        if (!first) await this.registerAgain();
        first = false;
        await this.session();
      } catch (error) {
        this.emit('error', error instanceof Error ? error : new Error(String(error)));
      }
      if (this.stopped) break;
      attempt = Date.now() - started > 60_000 ? 1 : attempt + 1;
      const base = Math.min(MAX_BACKOFF_MS, 1000 * 2 ** (attempt - 1));
      const delay = base * (0.9 + Math.random() * 0.2);
      this.log?.warn(`[tawks] reconnecting in ${Math.round(delay / 1000)}s`);
      await new Promise<void>((resolve) => {
        this.wake = resolve;
        setTimeout(resolve, delay);
      });
      this.wake = null;
    }
  }

  stop(): void {
    this.stopped = true;
    this.socket?.close(1000, 'stopping');
    this.wake?.();
  }

  private emit<K extends keyof Events>(event: K, ...args: Parameters<Events[K]>): void {
    if (event === 'error' && this.listeners.error.size === 0) {
      this.log?.error('[tawks]', (args[0] as Error).message);
      return;
    }
    for (const listener of this.listeners[event]) {
      try {
        (listener as (...a: Parameters<Events[K]>) => void)(...args);
      } catch (error) {
        this.log?.error('[tawks] listener failed', error);
      }
    }
  }

  private async registerAgain(): Promise<void> {
    for (const [actionId, id] of this.actionIds) {
      const again = await this.api.mutate<string>('server_action_register', {
        action_id: actionId,
        registrar_id: this.registrarId,
      });
      if (again !== id) {
        this.handlers.set(again, this.handlers.get(id)!);
        this.actionIds.set(actionId, again);
      }
    }
    for (const definition of this.workflows.values()) await this.api.mutate('workflow_register_workflow', definition);
    for (const activity of this.activities.values()) await this.api.mutate('activity_register', activity);
    this.log?.info('[tawks] registered everything again');
  }

  private async session(): Promise<void> {
    const url = new URL(this.eventsUrl);
    url.searchParams.set('token', await this.api.accessToken());
    const socket = new WebSocket(url);
    this.socket = socket;

    await new Promise<void>((resolve, reject) => {
      const timers: ReturnType<typeof setInterval>[] = [];
      const finish = (reason: string) => {
        timers.forEach(clearInterval);
        this.socket = null;
        this.emit('disconnect', reason);
        resolve();
      };
      socket.addEventListener('open', () => {
        this.log?.info('[tawks] connected');
        this.emit('connect');
        timers.push(
          setInterval(() => socket.send(JSON.stringify({ type: 'heartbeat', niche_id: null, channel_id: null })), HEARTBEAT_MS),
          setInterval(() => void this.renew(socket), RENEW_MS),
        );
      });
      socket.addEventListener('message', (event) => void this.receive(String(event.data)));
      socket.addEventListener('close', (event) => finish(event.reason || `closed with ${event.code}`));
      socket.addEventListener('error', () => {
        if (socket.readyState !== WebSocket.OPEN) reject(new Error('could not open the events socket'));
      });
    });
  }

  /** Leases lapse after about 45 seconds, and an API restart drops them all. */
  private async renew(socket: WebSocket): Promise<void> {
    try {
      const renewed = await this.api.mutate<number>('server_action_renew_leases', { registrar_id: this.registrarId });
      if (renewed < this.actionIds.size) {
        this.log?.warn('[tawks] server action leases were lost; reconnecting to register again');
        socket.close(4000, 'leases lost');
        return;
      }
      if (this.workflows.size === 0) return;
      const rows = await this.api.query<{ workflow_id: string; available_now: boolean }[]>(
        'workflow_definition_availability',
        { workflow_ids: [...this.workflows.keys()] },
      );
      const missing = rows.find((row) => !row.available_now);
      if (missing) {
        this.log?.warn(`[tawks] ${missing.workflow_id} is gone from tawks; reconnecting to register again`);
        socket.close(4000, 'workflow lost');
      }
    } catch (error) {
      this.log?.warn('[tawks] lease renewal failed', error instanceof Error ? error.message : error);
    }
  }

  private async receive(text: string): Promise<void> {
    let message: BotMessage;
    try {
      message = JSON.parse(text);
    } catch {
      return;
    }
    if (message.type === 'workflow_created' || message.type === 'workflow_updated') return;
    if (message.type !== 'workflow_request_server_action') {
      this.emit('message', message);
      return;
    }
    const { id, action_id, workflow } = message as unknown as {
      id: string;
      action_id: string;
      workflow: WorkflowResourceWithContext;
    };
    const now = Date.now();
    for (const [token, at] of this.seenTokens) if (now - at > DEDUPE_MS) this.seenTokens.delete(token);
    if (this.seenTokens.has(id)) return;
    this.seenTokens.set(id, now);

    const handler = this.handlers.get(action_id);
    if (!handler) {
      this.emit('message', message);
      return;
    }
    let outcome: ServerActionResult;
    try {
      outcome = await handler(new ServerActionContext(workflow));
    } catch (error) {
      outcome = result.error(error instanceof Error ? error.message : String(error));
    }
    try {
      await this.api.mutate('workflow_respond_server_action', { token: id, result: outcome });
    } catch (error) {
      const text = error instanceof Error ? error.message : String(error);
      if (!/No external action context|Workflow instance not found/.test(text)) {
        this.log?.warn(`[tawks] could not answer ${action_id}: ${text}`);
      }
    }
  }
}
