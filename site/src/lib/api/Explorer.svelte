<script lang="ts">
  import { onMount } from 'svelte';
  import { toast } from 'svelte-sonner';
  import { Copy, Send } from '@lucide/svelte';
  import { procedures, type Procedure } from './procedures';

  const targets = [
    { key: 'tawks', label: 'tawks.app', url: 'https://rspc.tawks.app/rspc' },
    { key: 'devhost', label: 'devhost on this machine', url: 'http://localhost:8787/rspc' },
  ];
  const groups = [...new Set(procedures.map((p) => p.group))];

  let selected = $state<Procedure>(procedures[0]);
  let filter = $state('');
  let target = $state('tawks');
  let token = $state('');
  let input = $state('');
  let inputError = $state<string | null>(null);
  let sending = $state(false);
  let response = $state<{ status: number; ms: number; body: string; ok: boolean } | null>(null);

  const url = $derived(`${targets.find((t) => t.key === target)!.url}/${selected.name}`);
  const visible = $derived(
    procedures.filter((p) => !filter || `${p.name} ${p.summary}`.toLowerCase().includes(filter.toLowerCase())),
  );
  const authorization = $derived(`Bot ${token.trim() || '<token>'}`);

  function pick(procedure: Procedure, push = true) {
    selected = procedure;
    input = JSON.stringify(procedure.input, null, 2);
    inputError = null;
    response = null;
    if (push) history.replaceState(null, '', `#${procedure.name}`);
  }

  function parsedInput(): unknown {
    try {
      inputError = null;
      return input.trim() ? JSON.parse(input) : null;
    } catch (e) {
      inputError = e instanceof Error ? e.message : String(e);
      return undefined;
    }
  }

  async function send() {
    const body = parsedInput();
    if (body === undefined) return;
    if (!token.trim()) {
      toast.warning('Paste a bot token first. On devhost any name works.');
      return;
    }
    sending = true;
    const started = performance.now();
    try {
      const res = await fetch(url, {
        method: 'POST',
        credentials: 'omit',
        headers: { authorization, 'content-type': 'application/json' },
        body: JSON.stringify(body),
      });
      const text = await res.text();
      let pretty = text;
      let ok = res.ok;
      try {
        const json = JSON.parse(text);
        const inner = json.result ?? json;
        ok = ok && inner.type === 'response';
        pretty = JSON.stringify(inner.type === 'response' ? inner.data : inner, null, 2);
      } catch {
        // Not JSON; show it as it came.
      }
      response = { status: res.status, ms: Math.round(performance.now() - started), body: pretty, ok };
    } catch {
      response = {
        status: 0,
        ms: Math.round(performance.now() - started),
        body:
          target === 'devhost'
            ? 'devhost did not answer. Is it running on port 8787, and up to date?'
            : 'The request did not reach tawks. Check your connection.',
        ok: false,
      };
    } finally {
      sending = false;
    }
  }

  function copy(text: string, what: string) {
    navigator.clipboard.writeText(text).then(
      () => toast.success(`${what} copied.`),
      () => toast.error('The clipboard is blocked here; select the text and copy it.'),
    );
  }

  const shellQuote = (s: string) => `'${s.replace(/'/g, `'\\''`)}'`;

  function asCurl() {
    const body = parsedInput();
    if (body === undefined) return;
    copy(
      `curl -X POST ${url} \\\n  -H ${shellQuote(`authorization: ${authorization}`)} \\\n  -H 'content-type: application/json' \\\n  -d ${shellQuote(JSON.stringify(body))}`,
      'curl command',
    );
  }

  function asJs() {
    const body = parsedInput();
    if (body === undefined) return;
    copy(`const result = await bot.api.${selected.kind}('${selected.name}', ${JSON.stringify(body, null, 2)});`, 'JavaScript');
  }

  onMount(() => {
    try {
      token = sessionStorage.getItem('explorer.token') ?? '';
      target = sessionStorage.getItem('explorer.target') ?? 'tawks';
    } catch {
      // Storage can be blocked; the fields still work.
    }
    const fromHash = () => procedures.find((p) => p.name === location.hash.slice(1));
    pick(fromHash() ?? procedures[0], false);
    const onHash = () => {
      const p = fromHash();
      if (p && p.name !== selected.name) pick(p, false);
    };
    window.addEventListener('hashchange', onHash);
    return () => window.removeEventListener('hashchange', onHash);
  });

  $effect(() => {
    try {
      sessionStorage.setItem('explorer.token', token);
      sessionStorage.setItem('explorer.target', target);
    } catch {
      // Storage can be blocked.
    }
  });
</script>

{#snippet prose(text: string)}
  {#each text.split('`') as part, i (i)}{#if i % 2}<code class="rounded bg-card px-1 font-mono text-xs text-foreground">{part}</code>{:else}{part}{/if}{/each}
{/snippet}

<div class="grid min-h-[calc(100vh-3.5rem)] grid-cols-1 lg:grid-cols-[18rem_minmax(0,1fr)]">
  <aside class="flex flex-col border-b lg:border-r lg:border-b-0">
    <div class="border-b p-3">
      <input class="h-8 w-full rounded-md border bg-card px-2 text-sm" placeholder="Filter procedures" bind:value={filter} />
    </div>
    <nav class="max-h-72 overflow-y-auto p-2 text-sm lg:max-h-none lg:flex-1">
      {#each groups as group (group)}
        {@const items = visible.filter((p) => p.group === group)}
        {#if items.length}
          <div class="mt-3 mb-1 px-2 text-xs font-semibold uppercase tracking-wide text-muted-foreground first:mt-0">{group}</div>
          {#each items as p (p.name)}
            <button
              class="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left font-mono text-xs {selected.name === p.name ? 'bg-secondary text-foreground' : 'text-muted-foreground hover:text-foreground'}"
              onclick={() => pick(p)}
            >
              <span class="min-w-0 flex-1 truncate">{p.name}</span>
              <span class="shrink-0 text-[10px] uppercase {p.kind === 'query' ? 'text-sky-400' : 'text-primary'}">{p.kind === 'query' ? 'Q' : 'M'}</span>
            </button>
          {/each}
        {/if}
      {/each}
    </nav>
  </aside>

  <section class="min-w-0 p-4 md:p-6">
    <div class="mx-auto flex max-w-3xl flex-col gap-4">
      <div>
        <div class="flex flex-wrap items-center gap-2">
          <h1 class="font-mono text-xl font-semibold">{selected.name}</h1>
          <span class="rounded border px-1.5 py-0.5 text-xs text-muted-foreground">{selected.kind}</span>
        </div>
        <p class="mt-2 text-sm text-muted-foreground">{@render prose(selected.summary)}</p>
        <p class="mt-1 text-sm text-muted-foreground">
          Returns {@render prose(selected.returns)}.
          {#if selected.doc}More in <a class="text-primary" href="/docs/{selected.doc[1]}">{selected.doc[0]}</a>.{/if}
        </p>
      </div>

      <div class="grid gap-3 sm:grid-cols-[minmax(0,14rem)_minmax(0,1fr)]">
        <label class="flex flex-col gap-1 text-xs text-muted-foreground">
          Send to
          <select class="h-8 rounded-md border bg-card px-2 text-sm text-foreground" bind:value={target}>
            {#each targets as t (t.key)}<option value={t.key}>{t.label}</option>{/each}
          </select>
        </label>
        <label class="flex flex-col gap-1 text-xs text-muted-foreground">
          Bot token, kept in this tab only
          <input
            class="h-8 rounded-md border bg-card px-2 font-mono text-sm text-foreground"
            type="password"
            autocomplete="off"
            spellcheck="false"
            placeholder={target === 'devhost' ? 'any name, such as mybot' : 'tbot_...'}
            bind:value={token}
          />
        </label>
      </div>

      <div class="flex flex-col gap-1">
        <div class="flex items-center gap-2 text-xs text-muted-foreground">
          <span class="flex-1 truncate font-mono">POST {url}</span>
          <button class="flex items-center gap-1 hover:text-foreground" onclick={asCurl}><Copy class="size-3.5" /> curl</button>
          <button class="flex items-center gap-1 hover:text-foreground" onclick={asJs}><Copy class="size-3.5" /> JavaScript</button>
        </div>
        <textarea
          class="min-h-48 resize-y rounded-md border bg-card p-3 font-mono text-xs"
          spellcheck="false"
          bind:value={input}
          oninput={() => (inputError = null)}
        ></textarea>
        {#if inputError}<p class="text-xs text-destructive">{inputError}</p>{/if}
      </div>

      <div class="flex items-center gap-3">
        <button
          class="flex items-center gap-2 rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground disabled:opacity-60"
          disabled={sending}
          onclick={send}
        >
          <Send class="size-4" /> {sending ? 'Sending' : 'Send'}
        </button>
        {#if selected.kind === 'mutation' && target === 'tawks'}
          <span class="text-xs text-muted-foreground">This changes things on tawks as your bot.</span>
        {/if}
      </div>

      {#if response}
        <div class="flex flex-col gap-1">
          <div class="flex items-center gap-3 text-xs">
            <span class={response.ok ? 'text-primary' : 'text-destructive'}>{response.ok ? 'OK' : 'Error'}</span>
            {#if response.status}<span class="text-muted-foreground">HTTP {response.status}</span>{/if}
            <span class="text-muted-foreground">{response.ms} ms</span>
          </div>
          <pre class="max-h-[32rem] overflow-auto rounded-md border bg-card p-3 font-mono text-xs">{response.body}</pre>
        </div>
      {/if}
    </div>
  </section>
</div>
