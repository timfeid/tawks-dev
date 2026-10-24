<script lang="ts">
  import { onMount } from 'svelte';
  import { toast } from 'svelte-sonner';
  import { Gamepad2, Link2, Play as PlayIcon } from '@lucide/svelte';
  import type { CreateWorkflowDefinition, WorkflowNode, WorkflowResource } from '@tawks/workflow-types';
  import Preview from '../playground/Preview.svelte';
  import { nextNode } from '../playground/runtime';
  import { DEFAULT_SANDBOX, Sandbox, newSession, type Activity, type Instance, type LogLine } from './sandbox';

  let sandbox = $state<Sandbox | null>(null);
  let activities = $state<Activity[]>([]);
  let definitions = $state<Record<string, CreateWorkflowDefinition | null>>({});
  let instances = $state<Instance[]>([]);
  let logs = $state<LogLine[]>([]);
  let gameId = $state('');
  let seats = $state(2);
  let starting = $state(false);
  let loading = $state(true);
  let unreachable = $state<string | null>(null);

  // Spelled out so Tailwind sees every class.
  const WIDE_COLUMNS = ['', 'xl:grid-cols-1', 'xl:grid-cols-2', 'xl:grid-cols-3', 'xl:grid-cols-4'];

  const game = $derived(activities.find((a) => a.workflow_id === gameId) ?? null);
  const minSeats = $derived(Math.max(1, game?.min_players ?? 1));
  const maxSeats = $derived(Math.min(8, game?.max_players ?? 8));
  const seated = $derived([...instances].sort((a, b) => a.user_id.localeCompare(b.user_id, undefined, { numeric: true })));

  function resourceFor(instance: Instance): WorkflowResource | null {
    const definition = definitions[instance.workflow_id];
    if (!definition) return null;
    const nodeId = nextNode(definition, '', { responses: instance.responses as never, context: instance.context as never });
    const node = nodeId ? (definition.nodes[nodeId] as WorkflowNode) : undefined;
    return {
      inject: null,
      instance_id: instance.id,
      workflow_id: instance.workflow_id,
      stale_definition: false,
      name: definition.name,
      description: definition.description ?? null,
      responses: instance.responses as WorkflowResource['responses'],
      completed: instance.completed,
      status: instance.completed ? 'completed' : 'active',
      complete_message: instance.complete_message,
      visuals: node?.visuals ?? [],
      layout: node?.layout ?? null,
      user_id: instance.user_id,
      current_node_id: nodeId ?? '',
      waiting: false,
      attached_workflows: {},
      parent: null,
      created_at: '',
      updated_at: '',
      completed_at: null,
    };
  }

  /** Views carry an increasing $rev; an older one must never replace a newer one. */
  function revision(instance: Instance): number {
    const rev = instance.responses.$rev;
    return typeof rev === 'number' ? rev : 0;
  }

  function playerName(instance: Instance): string {
    const user = instance.context.$user as { display_name?: string } | undefined;
    return user?.display_name ?? instance.user_id;
  }

  async function act(instance: Instance, actionId: string, inputs: Record<string, unknown>) {
    try {
      await sandbox?.act(instance.id, actionId, inputs);
    } catch (e) {
      toast.error(e instanceof Error ? e.message : String(e));
      throw e;
    }
  }

  async function findActivity(workflowId: string) {
    const found = activities.find((a) => a.workflow_id === workflowId);
    return found ? { workflow_id: found.workflow_id, name: found.name, client_url: found.client_url } : null;
  }

  async function start() {
    if (!sandbox || !game) return;
    starting = true;
    logs = [];
    try {
      // The start's own answer holds each seat as it was when that player sat
      // down; the stream has newer views by now, so ask for the lobby as it is.
      await sandbox.start(game.workflow_id, seats);
      instances = (await sandbox.state()).instances;
    } catch (e) {
      toast.error(e instanceof Error ? e.message : String(e));
    } finally {
      starting = false;
    }
  }

  function share() {
    if (!sandbox) return;
    const query = sandbox.url === DEFAULT_SANDBOX ? '' : `?sandbox=${encodeURIComponent(sandbox.url)}`;
    const url = `${location.origin}/play${query}#lobby=${sandbox.session}`;
    navigator.clipboard.writeText(url).then(
      () => toast.success('Link copied. Whoever opens it sees this lobby and can play any seat.'),
      () => toast.error('The clipboard is blocked here; copy the address bar instead.'),
    );
  }

  function pickGame(id: string) {
    gameId = id;
    const picked = activities.find((a) => a.workflow_id === id);
    seats = Math.min(Math.max(seats, picked?.min_players ?? 1), picked?.max_players ?? 8);
  }

  onMount(() => {
    const params = new URLSearchParams(location.search);
    const url = params.get('sandbox') ?? DEFAULT_SANDBOX;
    const fromLink = new URLSearchParams(location.hash.slice(1)).get('lobby');
    let session = fromLink;
    if (!session) {
      try {
        session = localStorage.getItem('play.lobby');
      } catch {
        session = null;
      }
    }
    session ??= newSession();
    try {
      localStorage.setItem('play.lobby', session);
    } catch {
      // Without storage a reload just opens a new lobby.
    }
    history.replaceState(null, '', `${location.pathname}${location.search}#lobby=${session}`);
    const client = new Sandbox(url, session);
    sandbox = client;

    client
      .state()
      .then((state) => {
        activities = state.activities;
        definitions = state.definitions;
        instances = state.instances;
        const running = state.instances[0]?.workflow_id;
        pickGame(running ?? state.activities[0]?.workflow_id ?? '');
        if (running) seats = state.instances.length;
      })
      .catch((e) => (unreachable = e instanceof Error ? e.message : String(e)))
      .finally(() => (loading = false));

    return client.listen({
      instance: (changed) => {
        const current = instances.find((i) => i.id === changed.id);
        if (current && revision(current) > revision(changed)) return;
        instances = [...instances.filter((i) => i.id !== changed.id), changed];
      },
      activities: (list) => {
        activities = list;
        void client.state().then((state) => (definitions = state.definitions));
      },
      log: (line) => (logs = [...logs, line].slice(-100)),
    });
  });
</script>

<div class="flex min-h-[calc(100vh-3.5rem)] flex-col">
  <div class="flex flex-wrap items-center gap-3 border-b px-4 py-3">
    <Gamepad2 class="size-5 text-primary" />
    {#if activities.length}
      <select
        class="h-9 min-w-0 max-w-xs flex-1 rounded-md border bg-card px-2 text-sm"
        value={gameId}
        onchange={(e) => pickGame((e.currentTarget as HTMLSelectElement).value)}
      >
        {#each activities as a (a.workflow_id)}<option value={a.workflow_id}>{a.name}</option>{/each}
      </select>
      <label class="flex items-center gap-2 text-sm text-muted-foreground">
        Seats
        <input class="h-9 w-16 rounded-md border bg-card px-2 text-sm text-foreground" type="number" min={minSeats} max={maxSeats} bind:value={seats} />
      </label>
      <button
        class="flex h-9 items-center gap-2 rounded-md bg-primary px-4 text-sm font-medium text-primary-foreground disabled:opacity-60"
        disabled={starting || !game}
        onclick={start}
      >
        <PlayIcon class="size-4" />
        {instances.length ? 'New game' : 'Start'}
      </button>
      <button class="flex h-9 items-center gap-1 rounded-md border px-3 text-sm text-muted-foreground hover:text-foreground" onclick={share}>
        <Link2 class="size-4" /> Copy link
      </button>
      {#if game?.description}<span class="text-sm text-muted-foreground">{game.description}</span>{/if}
    {/if}
  </div>

  {#if loading}
    <p class="p-8 text-sm text-muted-foreground">Connecting to the sandbox...</p>
  {:else if unreachable}
    <div class="mx-auto max-w-lg p-8 text-sm text-muted-foreground">
      <p class="text-foreground">{unreachable}</p>
      <p class="mt-2">The sandbox may be restarting. Try again in a minute.</p>
    </div>
  {:else if !activities.length}
    <div class="mx-auto max-w-lg p-8 text-sm text-muted-foreground">
      <p class="text-foreground">No games are connected to the sandbox right now.</p>
      <p class="mt-2">
        Connect your own: make a sandbox bot at <a class="text-primary" href="{sandbox?.url}/sandbox/token">{sandbox?.url}/sandbox/token</a>
        and run your bot against the sandbox.
      </p>
    </div>
  {:else if !instances.length}
    <div class="mx-auto max-w-lg p-8 text-sm text-muted-foreground">
      <p class="text-foreground">Pick a game and press Start.</p>
      <p class="mt-2">Every seat appears side by side, so you can play them all yourself, or copy the link and play with someone else.</p>
    </div>
  {:else}
    <div class="grid flex-1 grid-cols-1 gap-3 p-3 md:grid-cols-2 {WIDE_COLUMNS[Math.min(seated.length, 4)]}">
      {#each seated as instance (instance.id)}
        {@const resource = resourceFor(instance)}
        <section class="flex min-h-[28rem] flex-col overflow-hidden rounded-lg border bg-background">
          <div class="border-b bg-card px-3 py-1.5 text-xs font-medium">{playerName(instance)}</div>
          <div class="relative min-h-0 flex-1">
            {#if resource}
              <Preview {resource} onAction={(actionId, inputs) => act(instance, actionId, inputs)} {findActivity} />
            {:else}
              <p class="p-4 text-sm text-muted-foreground">This game's page is not registered right now.</p>
            {/if}
          </div>
        </section>
      {/each}
    </div>
    {#if logs.length}
      <ol class="max-h-40 overflow-y-auto border-t px-4 py-2 font-mono text-xs text-muted-foreground">
        {#each logs as line, i (i)}<li class={line.level === 'error' ? 'text-destructive' : ''}>{line.text}</li>{/each}
      </ol>
    {/if}
  {/if}
</div>
