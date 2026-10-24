<script lang="ts">
  import type { CardView, MatchStateView } from '../types';

  let {
    mainDeck = [],
    sideboard = [],
    matchState,
    viewerId,
    onSubmit,
  }: {
    /** Cards currently in the viewer's main deck. Counted by template_id. */
    mainDeck: CardView[];
    /** Cards currently in the viewer's sideboard pool. Counted by template_id. */
    sideboard: CardView[];
    matchState?: MatchStateView | null;
    viewerId: string | null;
    /** Fires when the viewer confirms their swap selection. */
    onSubmit: (swaps: { main_out: string; side_in: string }[]) => void;
  } = $props();

  type CardGroup = { template_id: string; name: string; image_url?: string; count: number };

  function groupByTemplate(cards: CardView[]): CardGroup[] {
    const byId = new Map<string, CardGroup>();
    for (const card of cards) {
      const key = (card as any).template_id || card.id;
      const existing = byId.get(key);
      if (existing) {
        existing.count += 1;
      } else {
        byId.set(key, {
          template_id: key,
          name: card.name,
          image_url: (card as any).image_url || undefined,
          count: 1,
        });
      }
    }
    return Array.from(byId.values()).sort((a, b) => a.name.localeCompare(b.name));
  }

  // Pending swaps, keyed by main-out → side-in counts. The UI lets the
  // player queue multiple swaps; each side-in is paired 1-for-1 with a
  // main-out pulled from the main deck.
  let pendingSwaps = $state<{ main_out: string; side_in: string }[]>([]);

  // Tentative selection state: once a player clicks a sideboard card, we
  // prompt them to pick which main-deck card to swap it for.
  let stagedSideIn = $state<string | null>(null);

  let mainGroups = $derived(groupByTemplate(mainDeck));
  let sideGroups = $derived(groupByTemplate(sideboard));

  let submitted = $derived.by(() => {
    if (!matchState || !viewerId) return false;
    return !matchState.pending_sideboard.includes(viewerId);
  });

  // Post-swap counts reflect pendingSwaps applied to main/side groups.
  let adjustedMain = $derived.by(() => {
    const counts = new Map(mainGroups.map((g) => [g.template_id, g.count]));
    for (const swap of pendingSwaps) {
      counts.set(swap.main_out, (counts.get(swap.main_out) ?? 0) - 1);
      counts.set(swap.side_in, (counts.get(swap.side_in) ?? 0) + 1);
    }
    return counts;
  });

  let adjustedSide = $derived.by(() => {
    const counts = new Map(sideGroups.map((g) => [g.template_id, g.count]));
    for (const swap of pendingSwaps) {
      counts.set(swap.side_in, (counts.get(swap.side_in) ?? 0) - 1);
      counts.set(swap.main_out, (counts.get(swap.main_out) ?? 0) + 1);
    }
    return counts;
  });

  function allGroups(): CardGroup[] {
    const byId = new Map<string, CardGroup>();
    for (const g of mainGroups) byId.set(g.template_id, { ...g });
    for (const g of sideGroups) {
      const existing = byId.get(g.template_id);
      if (existing) existing.count += g.count;
      else byId.set(g.template_id, { ...g });
    }
    return Array.from(byId.values());
  }

  function pickSideIn(template_id: string) {
    if (submitted) return;
    const remaining = adjustedSide.get(template_id) ?? 0;
    if (remaining <= 0) return;
    stagedSideIn = template_id;
  }

  function pickMainOut(template_id: string) {
    if (submitted) return;
    if (!stagedSideIn) return;
    const remaining = adjustedMain.get(template_id) ?? 0;
    if (remaining <= 0) return;
    pendingSwaps = [...pendingSwaps, { main_out: template_id, side_in: stagedSideIn }];
    stagedSideIn = null;
  }

  function undoSwap(index: number) {
    if (submitted) return;
    pendingSwaps = pendingSwaps.filter((_, i) => i !== index);
  }

  function confirm() {
    onSubmit(pendingSwaps);
  }

  function skip() {
    onSubmit([]);
  }

  const all = $derived(allGroups());
</script>

<div class="pointer-events-auto fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-sm">
  <div class="flex max-h-[90vh] w-[min(1200px,95vw)] flex-col overflow-hidden rounded-xl border border-amber-500/40 bg-slate-950 shadow-[0_0_60px_rgba(251,191,36,0.25)]">
    <!-- Header -->
    <div class="flex items-center justify-between border-b border-white/10 bg-gradient-to-r from-amber-600/20 via-slate-900 to-slate-900 px-6 py-4">
      <div>
        <div class="font-mono text-xs uppercase tracking-widest text-amber-400">
          Sideboard — Game {matchState?.current_game ?? '?'} of {matchState?.wins_required ? matchState.wins_required * 2 - 1 : 3}
        </div>
        <div class="mt-1 text-2xl font-bold text-white">
          {submitted ? 'Waiting for opponent…' : stagedSideIn ? 'Now pick a main-deck card to swap OUT' : 'Pick a sideboard card to swap IN'}
        </div>
      </div>
      <div class="flex gap-2">
        {#if !submitted}
          <button
            class="rounded-md border border-slate-600 bg-slate-800 px-4 py-2 text-sm font-mono uppercase tracking-wider text-slate-200 transition hover:border-slate-400 hover:bg-slate-700"
            onclick={skip}
            disabled={pendingSwaps.length > 0}
            title={pendingSwaps.length > 0 ? 'Undo pending swaps first' : 'Submit no swaps'}
          >
            Skip
          </button>
          <button
            class="rounded-md border border-amber-400 bg-amber-600 px-4 py-2 text-sm font-mono uppercase tracking-wider text-white transition hover:bg-amber-500"
            onclick={confirm}
          >
            Confirm ({pendingSwaps.length} {pendingSwaps.length === 1 ? 'swap' : 'swaps'})
          </button>
        {/if}
      </div>
    </div>

    <!-- Pending swap list -->
    {#if pendingSwaps.length > 0}
      <div class="border-b border-white/5 bg-slate-900/60 px-6 py-3">
        <div class="mb-2 font-mono text-[10px] uppercase tracking-widest text-slate-400">Pending swaps</div>
        <div class="flex flex-wrap gap-2">
          {#each pendingSwaps as swap, i (i)}
            {@const outName = all.find((g) => g.template_id === swap.main_out)?.name ?? swap.main_out}
            {@const inName = all.find((g) => g.template_id === swap.side_in)?.name ?? swap.side_in}
            <button
              class="group flex items-center gap-2 rounded border border-slate-700 bg-slate-800 px-3 py-1.5 text-xs text-slate-200 transition hover:border-red-500 hover:bg-red-900/30"
              onclick={() => undoSwap(i)}
              title="Click to undo"
            >
              <span class="text-red-400">−</span>
              <span>{outName}</span>
              <span class="text-slate-500">→</span>
              <span class="text-green-400">+</span>
              <span>{inName}</span>
              <span class="ml-1 text-slate-500 opacity-0 transition group-hover:opacity-100">✕</span>
            </button>
          {/each}
        </div>
      </div>
    {/if}

    <!-- Columns -->
    <div class="grid flex-1 grid-cols-2 gap-0 overflow-hidden">
      <!-- Main deck -->
      <div class="flex flex-col border-r border-white/10 overflow-hidden">
        <div class="border-b border-white/10 bg-slate-900 px-4 py-2 font-mono text-[10px] uppercase tracking-widest text-slate-400">
          Main Deck
        </div>
        <div class="flex-1 overflow-y-auto p-3">
          <div class="grid grid-cols-2 gap-1.5">
            {#each mainGroups as group (group.template_id)}
              {@const remaining = adjustedMain.get(group.template_id) ?? 0}
              {@const highlight = !!stagedSideIn && remaining > 0 && !submitted}
              <button
                class="group flex items-center justify-between rounded border bg-slate-900 px-3 py-2 text-left text-sm transition {highlight
                  ? 'border-amber-500 bg-amber-500/10 cursor-pointer'
                  : 'border-slate-700 cursor-not-allowed'}"
                class:text-slate-300={remaining > 0}
                class:text-slate-600={remaining <= 0}
                disabled={!highlight}
                onclick={() => pickMainOut(group.template_id)}
              >
                <span class="truncate">{group.name}</span>
                <span class="ml-2 font-mono text-xs text-slate-500">×{remaining}</span>
              </button>
            {/each}
          </div>
        </div>
      </div>

      <!-- Sideboard -->
      <div class="flex flex-col overflow-hidden">
        <div class="border-b border-white/10 bg-slate-900 px-4 py-2 font-mono text-[10px] uppercase tracking-widest text-slate-400">
          Sideboard
        </div>
        <div class="flex-1 overflow-y-auto p-3">
          {#if sideGroups.length === 0}
            <div class="mt-10 text-center font-mono text-sm text-slate-500">No sideboard cards — this deck has none.</div>
          {:else}
            <div class="grid grid-cols-2 gap-1.5">
              {#each sideGroups as group (group.template_id)}
                {@const remaining = adjustedSide.get(group.template_id) ?? 0}
                {@const isStaged = stagedSideIn === group.template_id}
                {@const enabled = remaining > 0 && !submitted}
                <button
                  class="group flex items-center justify-between rounded border bg-slate-900 px-3 py-2 text-left text-sm transition {isStaged
                    ? 'border-amber-400 bg-amber-500/20'
                    : !isStaged && enabled
                      ? 'border-slate-700 hover:border-amber-500'
                      : 'border-slate-700'}"
                  class:text-slate-300={enabled}
                  class:text-slate-600={!enabled}
                  class:cursor-pointer={enabled}
                  class:cursor-not-allowed={!enabled}
                  disabled={!enabled}
                  onclick={() => pickSideIn(group.template_id)}
                >
                  <span class="truncate">{group.name}</span>
                  <span class="ml-2 font-mono text-xs text-slate-500">×{remaining}</span>
                </button>
              {/each}
            </div>
          {/if}
        </div>
      </div>
    </div>
  </div>
</div>
