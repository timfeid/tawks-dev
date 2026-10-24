<script lang="ts">
  import { onMount } from 'svelte';
  import { toast } from 'svelte-sonner';
  import { ArrowDown, ArrowUp, Copy, Link2, Plug, Plus, RotateCcw, Trash2 } from '@lucide/svelte';
  import type { CreateWorkflowDefinition, WorkflowNode, WorkflowResource } from '@tawks/workflow-types';
  import generated from './generated.json';
  import { act, begin, fromMocks, start, type Json, type LogEntry, type Mocks, type State } from './runtime';
  import { DEFAULT_DEVHOST, devhostActions, listWorkflows, type DevhostWorkflow } from './devhost';
  import { getAt, insertTarget, setAt, tree, type Path } from './tree';
  import { rustFor } from './codegen';
  import { decode, encode } from './share';
  import Outline from './Outline.svelte';
  import Preview from './Preview.svelte';

  type Template = (typeof generated.templates)[number];
  const templates = generated.templates as Template[];
  const palette = generated.palette;
  const user = { id: 'you', username: 'you', display_name: 'You', avatar_url: 'default/1.png', db_score: 0, is_bot: false };

  const clone = <T,>(v: T): T => JSON.parse(JSON.stringify(v));

  let templateKey = $state<string | null>(templates[0].key);
  let definition = $state<CreateWorkflowDefinition>(clone(templates[0].definition) as CreateWorkflowDefinition);
  let mocks = $state<Mocks>(clone(templates[0].mocks) as Mocks);
  let runtime = $state<State>(start(clone(templates[0].definition) as CreateWorkflowDefinition, user));
  let log = $state<LogEntry[]>([]);
  let run = $state(1);
  let busy = $state(false);

  let devhostOpen = $state(false);
  let devhostUrl = $state(DEFAULT_DEVHOST);
  let devhostWorkflows = $state<DevhostWorkflow[]>([]);
  let connecting = $state(false);
  /** The devhost workflow on screen, when server actions go to a real bot. */
  let live = $state<{ url: string; workflowId: string } | null>(null);
  const server = $derived(live ? devhostActions(live.url, live.workflowId) : fromMocks(mocks));

  let leftTab = $state<'build' | 'json' | 'rust'>('build');
  let rightTab = $state<'mocks' | 'state' | 'log'>('mocks');
  let selected = $state<Path | null>(null);

  let jsonText = $state('');
  let jsonError = $state<string | null>(null);
  let mocksText = $state('');
  let mocksError = $state<string | null>(null);
  let itemText = $state('');
  let itemError = $state<string | null>(null);

  const items = $derived(tree(definition as unknown as Json));
  const unchanged = $derived.by(() => {
    const t = templates.find((t) => t.key === templateKey);
    return t && JSON.stringify(t.definition) === JSON.stringify(definition) ? t.source : null;
  });
  const rust = $derived(rustFor(definition, unchanged));

  const resource = $derived.by((): WorkflowResource => {
    const node = runtime.nodeId ? (definition.nodes[runtime.nodeId] as WorkflowNode | undefined) : undefined;
    const now = new Date().toISOString();
    return {
      inject: null,
      instance_id: `playground-${run}`,
      workflow_id: `wf-${definition.id}-you`,
      stale_definition: false,
      name: definition.name,
      description: definition.description ?? null,
      responses: runtime.responses,
      completed: runtime.status === 'completed',
      status: runtime.status,
      complete_message: runtime.message,
      visuals: node?.visuals ?? [],
      layout: node?.layout ?? null,
      user_id: user.id,
      current_node_id: runtime.nodeId ?? '',
      waiting: false,
      attached_workflows: {},
      parent: null,
      created_at: now,
      updated_at: now,
      completed_at: null,
    };
  });

  function syncTexts() {
    jsonText = JSON.stringify(definition, null, 2);
    mocksText = JSON.stringify(mocks, null, 2);
    jsonError = null;
    mocksError = null;
    syncItem();
  }

  function syncItem() {
    const value = selected ? getAt(definition as unknown as Json, selected) : undefined;
    itemText = value === undefined ? '' : JSON.stringify(value, null, 2);
    itemError = null;
  }

  async function restart() {
    const def = clone(definition);
    runtime = start(def, user);
    run += 1;
    log = [];
    if (!def.starting_server_action) return;
    busy = true;
    try {
      const started = await begin(def, user, server);
      runtime = started.state;
      report(started.log);
    } finally {
      busy = false;
    }
  }

  function report(entries: LogEntry[]) {
    log = [...log, ...entries].slice(-200);
    const problem = entries.find((l) => l.kind === 'error' || l.kind === 'warn');
    if (problem) toast[problem.kind === 'error' ? 'error' : 'warning'](problem.text);
  }

  function loadTemplate(key: string) {
    if (key.startsWith('devhost:')) return loadDevhost(key.slice('devhost:'.length));
    const t = templates.find((t) => t.key === key);
    if (!t) return;
    live = null;
    templateKey = key;
    definition = clone(t.definition) as CreateWorkflowDefinition;
    mocks = clone(t.mocks) as Mocks;
    selected = null;
    syncTexts();
    restart();
  }

  /** Keeps the instance running when the definition changes, unless its node is gone. */
  function definitionChanged() {
    if (!runtime.nodeId || !definition.nodes[runtime.nodeId]) restart();
    syncTexts();
  }

  function loadDevhost(id: string) {
    const w = devhostWorkflows.find((w) => w.id === id);
    if (!w) return;
    live = { url: devhostUrl, workflowId: id };
    templateKey = `devhost:${id}`;
    definition = clone(w.definition);
    selected = null;
    syncTexts();
    restart();
  }

  async function connect() {
    connecting = true;
    try {
      devhostWorkflows = await listWorkflows(devhostUrl);
      try {
        localStorage.setItem('playground.devhost', devhostUrl);
      } catch {
        // Storage can be blocked; the default address still works.
      }
      if (devhostWorkflows.length) {
        toast.success(`Found ${devhostWorkflows.length} workflow${devhostWorkflows.length === 1 ? '' : 's'}. Pick one from the list.`);
        devhostOpen = false;
      } else {
        toast.warning('devhost is running, but no bot has registered a workflow yet.');
      }
    } catch (e) {
      toast.error(e instanceof Error ? e.message : String(e));
    } finally {
      connecting = false;
    }
  }

  async function onAction(actionId: string, inputs: Record<string, unknown>) {
    if (busy) return;
    busy = true;
    try {
      const result = await act(clone(definition), runtime, actionId, inputs as Record<string, Json>, server);
      runtime = result.state;
      report(result.log);
    } finally {
      busy = false;
    }
  }

  function applyJson() {
    try {
      const parsed = JSON.parse(jsonText);
      if (!parsed || typeof parsed !== 'object' || typeof parsed.nodes !== 'object') throw new Error('A definition needs a nodes object.');
      definition = parsed;
      jsonError = null;
      definitionChanged();
    } catch (e) {
      jsonError = e instanceof Error ? e.message : String(e);
    }
  }

  function applyMocks() {
    try {
      const parsed = JSON.parse(mocksText);
      if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) throw new Error('Mocks are an object of server action id to result.');
      mocks = parsed;
      mocksError = null;
    } catch (e) {
      mocksError = e instanceof Error ? e.message : String(e);
    }
  }

  function applyItem() {
    if (!selected) return;
    try {
      const next = clone(definition) as unknown as Json;
      setAt(next, selected, JSON.parse(itemText));
      definition = next as unknown as CreateWorkflowDefinition;
      itemError = null;
      definitionChanged();
    } catch (e) {
      itemError = e instanceof Error ? e.message : String(e);
    }
  }

  function select(path: Path) {
    selected = path;
    syncItem();
  }

  function addDisplay(display: unknown) {
    const target = insertTarget(definition as unknown as Json, selected, items);
    if (!target) {
      toast.warning('Add a node with a Block visual first.');
      return;
    }
    const next = clone(definition) as unknown as Json;
    const arr = getAt(next, target.array);
    if (!Array.isArray(arr)) return;
    arr.splice(target.index, 0, clone(display) as Json);
    definition = next as unknown as CreateWorkflowDefinition;
    selected = [...target.array, target.index];
    definitionChanged();
  }

  function moveSelected(delta: number) {
    if (!selected || typeof selected[selected.length - 1] !== 'number') return;
    const index = selected[selected.length - 1] as number;
    const next = clone(definition) as unknown as Json;
    const arr = getAt(next, selected.slice(0, -1));
    if (!Array.isArray(arr) || index + delta < 0 || index + delta >= arr.length) return;
    [arr[index], arr[index + delta]] = [arr[index + delta], arr[index]];
    definition = next as unknown as CreateWorkflowDefinition;
    selected = [...selected.slice(0, -1), index + delta];
    definitionChanged();
  }

  function removeSelected() {
    if (!selected || selected.length < 2) return;
    const next = clone(definition) as unknown as Json;
    const parent = getAt(next, selected.slice(0, -1));
    const key = selected[selected.length - 1];
    if (Array.isArray(parent) && typeof key === 'number') parent.splice(key, 1);
    else if (parent && typeof parent === 'object' && !Array.isArray(parent) && typeof key === 'string') {
      if (selected[0] === 'nodes' && selected.length === 2 && Object.keys(parent).length <= 1) {
        toast.warning('A workflow needs at least one node.');
        return;
      }
      delete (parent as Record<string, Json>)[key];
    }
    definition = next as unknown as CreateWorkflowDefinition;
    selected = null;
    definitionChanged();
  }

  function addNode() {
    const ids = Object.keys(definition.nodes);
    let n = ids.length + 1;
    while (definition.nodes[`node_${n}`]) n += 1;
    const id = `node_${n}`;
    const parent = runtime.nodeId ?? null;
    const node: WorkflowNode = {
      title: `Node ${n}`,
      description: null,
      layout: null,
      condition: null,
      parent_id: parent,
      visuals: [{ id: 'page', display: { Block: { class: 'mx-auto flex max-w-md flex-col gap-4 p-8', content: [] } } }],
    };
    definition = { ...definition, nodes: { ...definition.nodes, [id]: node } };
    selected = ['nodes', id];
    definitionChanged();
  }

  async function copy(text: string, what: string) {
    try {
      await navigator.clipboard.writeText(text);
      toast.success(`${what} copied.`);
    } catch {
      toast.error('The clipboard is blocked here; select the text and copy it.');
    }
  }

  function share() {
    const url = `${location.origin}${location.pathname}#p=${encode({ definition, mocks })}`;
    history.replaceState(null, '', url);
    copy(url, 'Link');
  }

  onMount(() => {
    try {
      devhostUrl = localStorage.getItem('playground.devhost') ?? DEFAULT_DEVHOST;
    } catch {
      devhostUrl = DEFAULT_DEVHOST;
    }
    const shared = decode(location.hash);
    if (shared) {
      templateKey = null;
      definition = shared.definition as CreateWorkflowDefinition;
      mocks = (shared.mocks as Mocks) ?? {};
      restart();
    }
    syncTexts();
  });
</script>

<div class="grid h-[calc(100vh-3.5rem)] min-h-0 grid-cols-1 lg:grid-cols-[22rem_minmax(0,1fr)_20rem]">
  <section class="flex min-h-0 flex-col border-r">
    <div class="flex items-center gap-2 border-b p-2">
      <select
        class="h-8 flex-1 rounded-md border bg-card px-2 text-sm"
        value={templateKey ?? ''}
        onchange={(e) => loadTemplate((e.currentTarget as HTMLSelectElement).value)}
      >
        {#if templateKey === null}<option value="">Shared page</option>{/if}
        <optgroup label="Templates">
          {#each templates as t (t.key)}<option value={t.key}>{t.name}: {t.description}</option>{/each}
        </optgroup>
        {#if devhostWorkflows.length}
          <optgroup label="From devhost">
            {#each devhostWorkflows as w (w.id)}<option value={`devhost:${w.id}`}>{w.definition.name} ({w.id})</option>{/each}
          </optgroup>
        {/if}
      </select>
      <button
        class="rounded-md border p-1.5 hover:text-foreground {live ? 'border-primary/60 text-primary' : 'text-muted-foreground'}"
        title="Run pages from your bot through devhost"
        onclick={() => (devhostOpen = !devhostOpen)}
      >
        <Plug class="size-4" />
      </button>
      <button class="rounded-md border p-1.5 text-muted-foreground hover:text-foreground" title="Copy a link to this page" onclick={share}>
        <Link2 class="size-4" />
      </button>
    </div>
    {#if devhostOpen}
      <form
        class="flex flex-col gap-2 border-b p-2 text-xs text-muted-foreground"
        onsubmit={(e) => {
          e.preventDefault();
          connect();
        }}
      >
        <p>
          Run devhost and your bot on this machine, then connect to load the pages your bot registered. Their server actions go to
          your bot.
        </p>
        <div class="flex gap-2">
          <input class="h-8 flex-1 rounded-md border bg-card px-2 text-sm text-foreground" bind:value={devhostUrl} spellcheck="false" />
          <button class="rounded-md border px-3 text-sm text-foreground hover:border-primary/60 disabled:opacity-60" disabled={connecting}>
            {connecting ? 'Connecting' : 'Connect'}
          </button>
        </div>
      </form>
    {/if}
    <div class="flex border-b text-sm">
      {#each [['build', 'Build'], ['json', 'JSON'], ['rust', 'Rust']] as [key, label] (key)}
        <button
          class="flex-1 px-3 py-2 {leftTab === key ? 'border-b-2 border-primary text-foreground' : 'text-muted-foreground'}"
          onclick={() => {
            leftTab = key as typeof leftTab;
            syncTexts();
          }}>{label}</button
        >
      {/each}
    </div>

    {#if leftTab === 'build'}
      <div class="flex min-h-0 flex-1 flex-col">
        <div class="min-h-0 flex-1 overflow-y-auto p-2">
          <Outline {items} {selected} onSelect={select} />
          <button class="mt-2 flex items-center gap-1 px-2 text-xs text-muted-foreground hover:text-foreground" onclick={addNode}>
            <Plus class="size-3" /> Add a node
          </button>
        </div>
        <div class="border-t p-2">
          <div class="mb-1 text-xs font-semibold uppercase tracking-wide text-muted-foreground">Add</div>
          <div class="flex flex-wrap gap-1">
            {#each palette as p (p.label)}
              <button class="rounded border px-2 py-1 text-xs hover:border-primary/60" title={p.hint} onclick={() => addDisplay(p.display)}>{p.label}</button>
            {/each}
          </div>
        </div>
        {#if selected}
          <div class="flex max-h-[45%] min-h-0 flex-col border-t p-2">
            <div class="mb-1 flex items-center gap-1">
              <span class="flex-1 truncate text-xs text-muted-foreground">{selected.join(' / ')}</span>
              <button class="rounded p-1 text-muted-foreground hover:text-foreground" title="Move up" onclick={() => moveSelected(-1)}><ArrowUp class="size-3.5" /></button>
              <button class="rounded p-1 text-muted-foreground hover:text-foreground" title="Move down" onclick={() => moveSelected(1)}><ArrowDown class="size-3.5" /></button>
              <button class="rounded p-1 text-muted-foreground hover:text-destructive" title="Delete" onclick={removeSelected}><Trash2 class="size-3.5" /></button>
            </div>
            <textarea
              class="min-h-32 flex-1 resize-none rounded-md border bg-card p-2 font-mono text-xs"
              spellcheck="false"
              bind:value={itemText}
              oninput={applyItem}
            ></textarea>
            {#if itemError}<p class="mt-1 text-xs text-destructive">{itemError}</p>{/if}
          </div>
        {/if}
      </div>
    {:else if leftTab === 'json'}
      <div class="flex min-h-0 flex-1 flex-col p-2">
        <textarea class="flex-1 resize-none rounded-md border bg-card p-2 font-mono text-xs" spellcheck="false" bind:value={jsonText} oninput={applyJson}></textarea>
        {#if jsonError}<p class="mt-1 text-xs text-destructive">{jsonError}</p>{/if}
      </div>
    {:else}
      <div class="flex min-h-0 flex-1 flex-col p-2">
        <p class="mb-2 text-xs text-muted-foreground">
          {unchanged ? 'The builder code for this template.' : 'Rust that loads exactly this page.'} Register it with
          <code>bot.register_workflow(definition())</code>.
        </p>
        <pre class="min-h-0 flex-1 overflow-auto rounded-md border bg-card p-2 font-mono text-xs">{rust}</pre>
        <button class="mt-2 flex items-center justify-center gap-1 rounded-md border py-1.5 text-sm hover:border-primary/60" onclick={() => copy(rust, 'Rust')}>
          <Copy class="size-4" /> Copy
        </button>
      </div>
    {/if}
  </section>

  <section class="flex min-h-0 flex-col">
    <div class="flex items-center gap-3 border-b px-3 py-2 text-xs text-muted-foreground">
      <span>Node <span class="text-foreground">{runtime.nodeId ?? 'none'}</span></span>
      <span>Status <span class="text-foreground">{runtime.status}</span></span>
      {#if runtime.message}<span class="truncate">"{runtime.message}"</span>{/if}
      <button class="ml-auto flex items-center gap-1 hover:text-foreground" onclick={restart}><RotateCcw class="size-3.5" /> Restart</button>
    </div>
    <div class="relative min-h-0 flex-1 overflow-auto bg-background">
      {#if runtime.nodeId}
        <Preview {resource} {onAction} />
      {:else}
        <p class="p-8 text-sm text-muted-foreground">No node to show. Add one under Build.</p>
      {/if}
    </div>
  </section>

  <section class="flex min-h-0 flex-col border-l">
    <div class="flex border-b text-sm">
      {#each [['mocks', live ? 'Bot' : 'Mocks'], ['state', 'State'], ['log', `Log${log.length ? ` (${log.length})` : ''}`]] as [key, label] (key)}
        <button
          class="flex-1 px-3 py-2 {rightTab === key ? 'border-b-2 border-primary text-foreground' : 'text-muted-foreground'}"
          onclick={() => {
            rightTab = key as typeof rightTab;
            if (key === 'mocks') mocksText = JSON.stringify(mocks, null, 2);
          }}>{label}</button
        >
      {/each}
    </div>
    {#if rightTab === 'mocks' && live}
      <div class="p-2 text-xs text-muted-foreground">
        <p>Server actions go to your bot through devhost at <code>{live.url}</code>, running <code>{live.workflowId}</code>.</p>
        <p class="mt-2">Edits to the page apply here only. Change your bot's code to keep them.</p>
        <button class="mt-3 rounded-md border px-3 py-1.5 text-sm text-foreground hover:border-primary/60" onclick={() => loadTemplate(templates[0].key)}>
          Back to templates
        </button>
      </div>
    {:else if rightTab === 'mocks'}
      <div class="flex min-h-0 flex-1 flex-col p-2">
        <p class="mb-2 text-xs text-muted-foreground">
          What each server action answers here, by action id, for example
          <code>{'{ "UpdateResponses": { "reply": "Hi" } }'}</code>. On tawks your bot answers instead.
        </p>
        <textarea class="flex-1 resize-none rounded-md border bg-card p-2 font-mono text-xs" spellcheck="false" bind:value={mocksText} oninput={applyMocks}></textarea>
        {#if mocksError}<p class="mt-1 text-xs text-destructive">{mocksError}</p>{/if}
      </div>
    {:else if rightTab === 'state'}
      <div class="min-h-0 flex-1 overflow-auto p-2">
        <div class="mb-1 text-xs font-semibold uppercase tracking-wide text-muted-foreground">Responses</div>
        <pre class="mb-3 rounded-md border bg-card p-2 font-mono text-xs">{JSON.stringify(runtime.responses, null, 2)}</pre>
        <div class="mb-1 text-xs font-semibold uppercase tracking-wide text-muted-foreground">Context</div>
        <pre class="rounded-md border bg-card p-2 font-mono text-xs">{JSON.stringify(runtime.context, null, 2)}</pre>
      </div>
    {:else}
      <ol class="min-h-0 flex-1 overflow-auto p-2 font-mono text-xs">
        {#each log as entry, i (i)}
          <li
            class="py-0.5 {entry.kind === 'error' ? 'text-destructive' : entry.kind === 'warn' ? 'text-amber-300' : entry.kind === 'server' ? 'text-primary' : 'text-muted-foreground'}"
          >
            {entry.kind === 'action' ? 'action ' : ''}{entry.text}
          </li>
        {:else}
          <li class="text-muted-foreground">Press something in the page.</li>
        {/each}
      </ol>
    {/if}
  </section>
</div>
