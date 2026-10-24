import type { CreateWorkflowDefinition } from '@tawks/workflow-types';

export const DEFAULT_SANDBOX = 'https://sandbox.tawks.app';

export type Activity = {
  workflow_id: string;
  name: string;
  client_url: string;
  description?: string | null;
  icon?: string | null;
  min_players?: number | null;
  max_players?: number | null;
  source_url?: string | null;
  bot?: string;
};

export type Instance = {
  id: string;
  session: string;
  workflow_id: string;
  user_id: string;
  responses: Record<string, unknown>;
  context: Record<string, unknown>;
  completed: boolean;
  complete_message: string | null;
  parent_instance_id?: string;
  inject?: unknown;
};

export type SandboxState = {
  activities: Activity[];
  definitions: Record<string, CreateWorkflowDefinition | null>;
  instances: Instance[];
  lobby_id: string;
};

export type LogLine = { level: string; text: string };

type PageEvent =
  | { type: 'instance'; instance: Instance }
  | { type: 'log'; level: string; text: string }
  | { type: 'activities'; activities: Activity[] };

const SOURCE = 'https://github.com/timfeid/tawks-dev/tree/main/';

/**
 * The open-source games whose players use workflow pages rather than an
 * activity frame. Like tawks' own game menu, the page knows them by their
 * workflow key, whichever bot registered them.
 */
const PAGE_GAMES = [
  { key: 'view_midnight_lobby', name: 'Midnight', description: 'A hidden-role party game', min: 1, max: 10, source: 'games/midnight' },
  { key: 'trivia_multiplayer', name: 'Trivia', description: 'Answer together, fastest scores most', min: 1, max: 8, source: 'games/trivia' },
  { key: 'tracklines_board', name: 'Tracklines', description: 'Claim railways across North America', min: 1, max: 5, source: 'games/tracklines' },
  { key: 'cardgame_table', name: 'Cards', description: 'Draft a deck and duel', min: 2, max: 8, source: 'games/cards' },
];

export type RegisteredWorkflow = { id: string; definition: CreateWorkflowDefinition };

/** The page games among the registered workflows, as entries for the game picker. */
export function pageGames(workflows: RegisteredWorkflow[]): Activity[] {
  return PAGE_GAMES.flatMap((game) =>
    workflows
      .filter((w) => w.id.startsWith(`wf-${game.key}-`))
      .map((w) => ({
        workflow_id: w.id,
        name: game.name,
        client_url: '',
        description: game.description,
        min_players: game.min,
        max_players: game.max,
        source_url: SOURCE + game.source,
      })),
  );
}

/** A lobby id of our own, kept so a reload or a shared link returns to the same lobby. */
export function newSession(): string {
  const bytes = crypto.getRandomValues(new Uint8Array(12));
  return Array.from(bytes, (b) => b.toString(16).padStart(2, '0')).join('');
}

export class Sandbox {
  readonly url: string;

  constructor(
    url: string,
    readonly session: string,
  ) {
    this.url = url.replace(/\/+$/, '');
  }

  private async request<T>(path: string, body?: unknown): Promise<T> {
    let res: Response;
    try {
      res = await fetch(`${this.url}${path}`, {
        method: body === undefined ? 'GET' : 'POST',
        headers: body === undefined ? undefined : { 'content-type': 'application/json' },
        body: body === undefined ? undefined : JSON.stringify(body),
      });
    } catch {
      throw new Error(`The sandbox at ${this.url} did not answer.`);
    }
    const data = await res.json().catch(() => null);
    if (!res.ok || (data && data.ok === false)) throw new Error(data?.error ?? `The sandbox answered ${res.status}.`);
    return data as T;
  }

  state(): Promise<SandboxState> {
    return this.request(`/dev/state?session=${this.session}`);
  }

  /** Every workflow the sandbox's bots registered, with its definition. */
  async workflows(): Promise<RegisteredWorkflow[]> {
    return (await this.request<{ workflows: RegisteredWorkflow[] }>('/dev/workflows')).workflows;
  }

  start(workflowId: string, players: number): Promise<{ instances: Instance[] }> {
    return this.request('/dev/start', { session: this.session, workflow_id: workflowId, players });
  }

  act(instanceId: string, action: string, inputs: Record<string, unknown>): Promise<unknown> {
    return this.request('/dev/act', { session: this.session, instance_id: instanceId, action, inputs });
  }

  /** Instance changes, activity registrations and log lines for this session, as they happen. */
  listen(handlers: {
    instance: (instance: Instance) => void;
    activities: (activities: Activity[]) => void;
    log: (line: LogLine) => void;
  }): () => void {
    const source = new EventSource(`${this.url}/dev/stream?session=${this.session}`);
    source.onmessage = (event) => {
      let message: PageEvent;
      try {
        message = JSON.parse(event.data);
      } catch {
        return;
      }
      if (message.type === 'instance') handlers.instance(message.instance);
      else if (message.type === 'activities') handlers.activities(message.activities);
      else if (message.type === 'log') handlers.log({ level: message.level, text: message.text });
    };
    return () => source.close();
  }
}
