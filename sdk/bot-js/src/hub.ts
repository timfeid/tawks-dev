import { TawksBot, type ServerActionHandler, type BotOptions } from './bot.js';
import type { CreateWorkflowDefinition, JsonValue } from '@tawks/workflow-types';
import type { TawksApiError } from './api.js';

/**
 * What a bot running as a hub's own bot gets on top of `TawksBot`: its SQLite
 * file, the hub's clock, wakes that survive a crash and a sleep, the hub's
 * members and connected data. Start here for a bot the hub agent writes.
 */

declare const process: {
  env: Record<string, string | undefined>;
  exit(code?: number): never;
};

type Sql = {
  exec(sql: string): void;
  prepare(sql: string): {
    run(...params: unknown[]): { changes: number | bigint; lastInsertRowid: number | bigint };
    get(...params: unknown[]): Record<string, any> | undefined;
    all(...params: unknown[]): Record<string, any>[];
  };
  close(): void;
};

const WAKE_POLL_MS = 1_000;
const FORGET_POLL_MS = 30_000;

/** Anything destroying what an earlier version wrote; `drops` on the change lets a table through. */
export function destructiveStatement(sql: string): string | null {
  const clean = sql
    .split('\n')
    .map((line) => line.split('--')[0])
    .join(' ')
    .toUpperCase();
  for (const statement of clean.split(';')) {
    const words = statement.trim().split(/\s+/);
    if (words[0] === 'DROP' || words[0] === 'TRUNCATE') return words.join(' ');
    if (words[0] === 'ALTER' && words[1] === 'TABLE' && (words[3] === 'DROP' || words[3] === 'RENAME')) return words.join(' ');
  }
  return null;
}

/**
 * The bot's SQLite file, in the hub's own volume. It survives crashes, sleeps,
 * new versions and rollbacks. Rollback runs older code on the same file, so
 * schema changes only add: new tables, new columns with defaults, new indexes.
 */
const PUSH_TRIES = 6;

export class HubDb {
  constructor(readonly sql: Sql) {
    sql.exec('PRAGMA journal_mode = WAL; PRAGMA synchronous = FULL; PRAGMA foreign_keys = ON; PRAGMA busy_timeout = 2000;');
  }

  /**
   * Brings the file up to date. Append to the list and never edit an entry. A
   * file ahead of this list (after a rollback) is left as it is. A statement
   * that drops or renames something is refused unless the change names it.
   */
  migrate(migrations: string[]): void {
    const allowed = (process.env.TAWKS_ALLOWED_DROPS ?? '').split(',').filter(Boolean).map((name) => name.toUpperCase());
    const version = Number(this.sql.prepare('PRAGMA user_version').get()?.user_version ?? 0);
    for (let index = version; index < migrations.length; index += 1) {
      const statement = destructiveStatement(migrations[index]);
      if (statement && !allowed.some((name) => statement.includes(name))) {
        throw new Error(`migration ${index + 1} would destroy data (${statement}); schema changes only add`);
      }
      this.tx(() => {
        this.sql.exec(migrations[index]);
        this.sql.exec(`PRAGMA user_version = ${index + 1}`);
      });
    }
  }

  run(sql: string, ...params: unknown[]) {
    return this.sql.prepare(sql).run(...params);
  }

  get<T = Record<string, any>>(sql: string, ...params: unknown[]): T | undefined {
    return this.sql.prepare(sql).get(...params) as T | undefined;
  }

  all<T = Record<string, any>>(sql: string, ...params: unknown[]): T[] {
    return this.sql.prepare(sql).all(...params) as T[];
  }

  /** Everything in `work` commits together or not at all. Not for async work. */
  tx<T>(work: () => T): T {
    this.sql.exec('BEGIN IMMEDIATE');
    try {
      const value = work();
      this.sql.exec('COMMIT');
      return value;
    } catch (error) {
      this.sql.exec('ROLLBACK');
      throw error;
    }
  }
}

/** The row fields each list on a page reads, by the response key the list shows. */
export function listFields(definition: unknown): Map<string, Set<string>> {
  const found = new Map<string, Set<string>>();
  const walk = (value: unknown): void => {
    if (Array.isArray(value)) return value.forEach(walk);
    if (value === null || typeof value !== 'object') return;
    const items = (value as Record<string, any>).Items;
    if (items && typeof items.items_key === 'string' && typeof items.as === 'string') {
      const fields = found.get(items.items_key) ?? new Set<string>();
      for (const match of JSON.stringify(items.content ?? []).matchAll(new RegExp(`\\{${items.as}\\.(\\w+)`, 'g'))) fields.add(match[1]);
      found.set(items.items_key, fields);
    }
    Object.values(value).forEach(walk);
  };
  walk(definition);
  return found;
}

/** Fields a list reads that its rows don't have. Each shows as a blank on the page. */
export function blankFields(fields: Map<string, Set<string>>, values: Partial<Record<string, JsonValue>>): string[] {
  const blanks: string[] = [];
  for (const [key, wanted] of fields) {
    const rows = values[key];
    if (!Array.isArray(rows) || rows.length === 0) continue;
    for (const field of wanted) {
      const missing = rows.some((row) => row !== null && typeof row === 'object' && !Array.isArray(row) && !(field in row));
      if (missing) blanks.push(`${key}.${field}`);
    }
  }
  return blanks;
}

/** A person, to a bot: an `m_...` id that is theirs in this hub and nowhere else. No name, ever. */
export type Member = { id: string; role: 'owner' | 'member' };
export type HubChannel = { id: string; name: string; slug: string; kind: string };
export type HubContext = { hub: { id: string; name: string; slug: string }; scope: 'live' | 'preview'; bot: string; channels: HubChannel[]; now: string };
export type DatasetResult = { rows: { id: string; data: Record<string, JsonValue> }[]; refused: string | null; notice: string | null; attribution: string };
export type Wake = { key: string; at: string; data: JsonValue };
export type WakeHandler = (wake: Wake) => void | Promise<void>;

export class HubBot {
  readonly bot: TawksBot;
  readonly db: HubDb;
  readonly scope: string;
  private readonly user: string;
  private readonly wakeHandlers = new Map<string, WakeHandler>();
  private forgetHandler: ((userId: string) => void | Promise<void>) | null = null;
  private latestNow = new Date();
  private running: Promise<void> | null = null;
  private readonly refreshes = new Map<string, { again: boolean; build: () => [string | null, Partial<Record<string, JsonValue>>][]; done: Promise<void> }>();
  private readonly lists = new Map<string, Set<string>>();
  private readonly warned = new Set<string>();

  constructor(db: HubDb, options: BotOptions) {
    this.bot = new TawksBot(options);
    this.db = db;
    this.scope = process.env.TAWKS_SCOPE ?? 'live';
    this.user = process.env.TAWKS_BOT_USER ?? '';
  }

  /** Registers a page. Returns its id, which `push` also takes as its key. */
  page(definition: CreateWorkflowDefinition): Promise<string> {
    for (const [key, fields] of listFields(definition)) {
      const known = this.lists.get(key) ?? new Set<string>();
      fields.forEach((field) => known.add(field));
      this.lists.set(key, known);
    }
    return this.bot.workflow(definition);
  }

  /** A list row with no value for what its page reads looks like an empty box, and nothing else says so. */
  private warnBlanks(values: Partial<Record<string, JsonValue>>): void {
    for (const blank of blankFields(this.lists, values)) {
      if (this.warned.has(blank)) continue;
      this.warned.add(blank);
      console.warn(`[hub] a page reads ${blank} but the rows sent have no such field, so it shows blank`);
    }
  }

  /** Registers a server action and returns the id its buttons target. */
  action(name: string, handler: ServerActionHandler): Promise<string> {
    return this.bot.serverAction(name, async (ctx) => {
      const answer = await handler(ctx);
      const responses = (answer as { UpdateResponses?: Partial<Record<string, JsonValue>> } | null)?.UpdateResponses;
      if (responses) this.warnBlanks(responses);
      return answer;
    });
  }

  /** Pushes values into every open copy of a page, or one person's. */
  async push(pageKey: string, updates: Partial<Record<string, JsonValue>>, userId?: string): Promise<unknown> {
    this.warnBlanks(updates);
    for (let attempt = 1; ; attempt += 1) {
      try {
        return await this.bot.api.mutate('workflow_broadcast_updates', {
          workflow_definition_id: `wf-${pageKey}-${this.user}`,
          filter: { user_id: userId ?? null, response_equals: null, include_completed: false },
          updates,
        });
      } catch (error) {
        // Someone pressing while a push lands is normal on a busy page. A push is
        // only values, so sending it again is safe, and failing would fail their press.
        if ((error as TawksApiError).code !== 409 || attempt === PUSH_TRIES) throw error;
        await new Promise((resolve) => setTimeout(resolve, 40 * attempt));
      }
    }
  }

  /**
   * Brings a live page up to date, however many changes land at once. `build`
   * reads the current state and says what to send: `[null, values]` for everyone
   * and `[userId, values]` for one person. Runs one at a time and once more if
   * anything changed meanwhile, so the last thing on the page is always the
   * latest. A plain push from each press can arrive out of order and stick.
   */
  async refresh(pageKey: string, build: () => [string | null, Partial<Record<string, JsonValue>>][]): Promise<void> {
    const queue = this.refreshes.get(pageKey);
    if (queue) {
      queue.again = true;
      queue.build = build;
      return queue.done;
    }
    const state = { again: false, build, done: Promise.resolve() };
    // Registered before the work starts: a round with nobody to push to finishes without
    // awaiting anything, and its cleanup must not run before this entry exists.
    this.refreshes.set(pageKey, state);
    state.done = (async () => {
      await Promise.resolve();
      let retries = 0;
      try {
        do {
          state.again = false;
          // One person's push failing must not cost everyone after them theirs.
          let failed = 0;
          for (const [userId, values] of state.build()) {
            try {
              await this.push(pageKey, values, userId ?? undefined);
            } catch (error) {
              failed += 1;
              console.warn(`[hub] a push to ${pageKey} failed: ${error instanceof Error ? error.message : error}`);
            }
          }
          if (failed && retries < 2) {
            retries += 1;
            state.again = true;
          }
        } while (state.again);
      } finally {
        this.refreshes.delete(pageKey);
      }
    })();
    return state.done;
  }

  /** The time by the hub's clock, which a preview or a test can move. Never use Date.now() for a deadline. */
  async now(): Promise<Date> {
    this.latestNow = new Date(await this.bot.api.query<string>('code_bot_now'));
    return this.latestNow;
  }

  /**
   * The time to give `ui.countdown` for a deadline by the hub's clock. A countdown
   * runs on the viewer's clock, which the hub's clock can differ from in a test or a
   * simulated season, so this says when the deadline falls on real clocks.
   */
  async countdownTo(deadline: Date | string): Promise<string> {
    const at = typeof deadline === 'string' ? new Date(deadline) : deadline;
    return new Date(Date.now() + (at.getTime() - (await this.now()).getTime())).toISOString();
  }

  /**
   * Wakes the bot at a time by the hub's clock, sleeping or not, and calls the
   * handler registered for the key or the part before its first dot. Setting a
   * key again moves it. It is kept outside the bot, so a crash doesn't lose it.
   */
  wakeAt(key: string, at: Date | string, data: JsonValue = null): Promise<unknown> {
    return this.bot.api.mutate('code_bot_wake_set', { key, at: typeof at === 'string' ? at : at.toISOString(), data });
  }

  cancelWake(key: string): Promise<unknown> {
    return this.bot.api.mutate('code_bot_wake_cancel', { key });
  }

  onWake(prefix: string, handler: WakeHandler): void {
    this.wakeHandlers.set(prefix, handler);
  }

  /** How to write this person in text, on a page or in chat. Tawks shows their current name to whoever reads it. */
  who(memberId: string): string {
    return `@[${memberId}]`;
  }

  /** Called with a member's id when they delete their account, to clear them from play. Their id already names nobody. */
  onForget(handler: (userId: string) => void | Promise<void>): void {
    this.forgetHandler = handler;
  }

  async context(): Promise<HubContext> {
    return JSON.parse(await this.bot.api.query<string>('code_bot_context'));
  }

  /** The hub's people, by id. In a preview, sample people. */
  async members(): Promise<Member[]> {
    const all: Member[] = [];
    let after: string | null = null;
    for (;;) {
      const page: { members: Member[]; next: string | null } = JSON.parse(
        await this.bot.api.query<string>('code_bot_members', { after, limit: 200 }),
      );
      all.push(...page.members);
      if (!page.next) return all;
      after = page.next;
    }
  }

  /** A dataset attached to the hub, read through its approval. Check `refused`, and show `attribution`. */
  async dataset(alias: string, limit = 1000): Promise<DatasetResult> {
    return JSON.parse(await this.bot.api.query<string>('code_bot_dataset', { alias, limit }));
  }

  /** A GET to a host this hub was approved for. Sockets to anywhere else don't exist in here. */
  async fetch(url: string): Promise<{ ok: boolean; body?: string; error?: string }> {
    return JSON.parse(await this.bot.api.mutate<string>('code_bot_fetch', { url }));
  }

  /** Runs a member action rule of one of the hub's systems, such as handing facts to its operator. */
  async act(system: string, rule: string, input: Partial<Record<string, JsonValue>> = {}): Promise<unknown> {
    return JSON.parse(await this.bot.api.mutate<string>('code_bot_system_act', { system, rule, input }));
  }

  /** Posts in one of the hub's chat channels. In a preview it goes to the log. */
  say(channelId: string, text: string): Promise<unknown> {
    return this.bot.say(channelId, text);
  }

  private async pollWakes(): Promise<void> {
    const answer: { now: string; due: Wake[] } = JSON.parse(await this.bot.api.mutate<string>('code_bot_wakes_due'));
    this.latestNow = new Date(answer.now);
    const done: string[] = [];
    for (const wake of answer.due) {
      const handler = this.wakeHandlers.get(wake.key) ?? this.wakeHandlers.get(wake.key.split('.')[0]);
      if (!handler) {
        console.warn(`nothing handles the wake ${wake.key}`);
        done.push(wake.key);
        continue;
      }
      try {
        await handler(wake);
        done.push(wake.key);
      } catch (error) {
        console.error(`the wake ${wake.key} failed and comes back in a minute:`, error);
      }
    }
    if (done.length) await this.bot.api.mutate('code_bot_wake_ack', { keys: done });
  }

  private async pollForgotten(): Promise<void> {
    if (!this.forgetHandler) return;
    const users = await this.bot.api.query<string[]>('code_bot_forgotten');
    for (const user of users) await this.forgetHandler(user);
    if (users.length) await this.bot.api.mutate('code_bot_forgotten_ack', { keys: users });
  }

  /**
   * Opens the events socket before anything registers. An action counts as
   * available from the moment it registers, so a press landing before the socket
   * is open would otherwise go to nobody, and a restart makes that window real.
   */
  async connect(): Promise<void> {
    await new Promise<void>((resolve) => {
      const stop = this.bot.on('connect', () => {
        stop();
        resolve();
      });
      this.running = this.bot.run();
      setTimeout(resolve, 5_000);
    });
  }

  /** Stays connected. Call last, once your actions and pages are registered. */
  async run(): Promise<void> {
    const loop = (work: () => Promise<void>, every: number) => {
      const step = () => {
        work()
          .catch((error) => console.warn('[hub]', error instanceof Error ? error.message : error))
          .finally(() => setTimeout(step, every));
      };
      step();
    };
    loop(() => this.pollWakes(), WAKE_POLL_MS);
    loop(() => this.pollForgotten(), FORGET_POLL_MS);
    await (this.running ?? this.bot.run());
  }
}

/** The hub's bot, connected through its sandbox's own settings, with its database open. */
export async function hubBot(): Promise<HubBot> {
  const { DatabaseSync } = (await import('node:sqlite' as string)) as { DatabaseSync: new (path: string) => Sql };
  const db = new HubDb(new DatabaseSync(process.env.TAWKS_DB_PATH ?? '/data/bot.sqlite'));
  const hub = new HubBot(db, {
    rspcUrl: process.env.RSPC_URL ?? '',
    eventsUrl: process.env.EVENTS_WS_URL ?? '',
    token: process.env.BOT_TOKEN ?? '',
  });
  await hub.connect();
  return hub;
}
