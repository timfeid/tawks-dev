<script lang="ts">
  import { onMount } from 'svelte';

  let {
    draftView,
    canAutoDraft = false,
    onAutoDraft = () => {},
  }: {
    draftView: any;
    canAutoDraft?: boolean;
    onAutoDraft?: () => void;
  } = $props();

  // Client-side countdown from server's pick_timer_seconds
  let countdown = $state(0);
  let lastPickerId = $state('');
  let lastServerTime = $state(0);

  $effect(() => {
    const pickerId = draftView.current_picker_id ?? '';
    const serverTime = draftView.pick_timer_seconds ?? 20;
    // Reset only when the server sends a new turn/timer baseline.
    if (pickerId !== lastPickerId || serverTime !== lastServerTime) {
      countdown = serverTime;
      lastPickerId = pickerId;
      lastServerTime = serverTime;
    }
  });

  onMount(() => {
    const interval = setInterval(() => {
      if (countdown > 0) countdown--;
    }, 1000);
    return () => clearInterval(interval);
  });

  let timerColor = $derived(
    countdown <= 5
      ? 'text-red-400'
      : countdown <= 10
        ? 'text-amber-400'
        : 'text-cyan-400',
  );

  let rarityEntries = $derived(
    Object.entries(draftView.opponent_rarity_counts || {}).filter(
      ([, count]) => (count as number) > 0,
    ),
  );
</script>

<!-- Top bar -->
<div class="pointer-events-none absolute left-0 top-0 z-40 flex w-full items-center justify-between px-6 py-4">
  <!-- Pack progress -->
  <div
    class="rounded-lg border border-slate-600/40 bg-slate-950/80 px-4 py-2 font-mono text-sm tracking-wider text-slate-300"
  >
    PACK {draftView.packs_completed + 1} / {draftView.total_packs}
  </div>

  <!-- Timer -->
  <div class="flex flex-col items-center gap-1">
    <span class="font-mono text-3xl font-bold tabular-nums {timerColor}">
      {countdown}s
    </span>
  </div>

  <!-- Drafted counts -->
  <div
    class="mr-40 flex gap-4 rounded-lg border border-slate-600/40 bg-slate-950/80 px-4 py-2 font-mono text-sm"
  >
    <span class="text-cyan-400">You: {draftView.my_drafted_count}</span>
    <span class="text-slate-500">|</span>
    <span class="text-slate-400">Opp: {draftView.opponent_drafted_count}</span>
    {#if canAutoDraft}
      <button
        class="pointer-events-auto rounded border px-2 py-1 text-[11px] font-semibold uppercase tracking-[0.14em] transition {draftView.auto_draft_enabled
          ? 'border-emerald-400/60 bg-emerald-500/25 text-emerald-100 hover:bg-emerald-500/35'
          : 'border-amber-400/40 bg-amber-500/15 text-amber-100 hover:bg-amber-500/25'}"
        onclick={onAutoDraft}
      >
        {draftView.auto_draft_enabled ? 'Auto: On' : 'Auto Draft'}
      </button>
    {/if}
  </div>
</div>

<!-- Turn indicator -->
<div class="pointer-events-none absolute left-1/2 top-16 z-40 -translate-x-1/2">
  {#if draftView.is_my_turn}
    <div
      class="animate-pulse rounded-lg border border-cyan-500/50 bg-slate-950/90 px-6 py-2 font-mono text-lg font-bold tracking-widest text-cyan-400 shadow-[0_0_20px_rgba(6,182,212,0.3)]"
    >
      YOUR PICK
    </div>
  {:else}
    <div
      class="rounded-lg border border-slate-600/30 bg-slate-950/90 px-6 py-2 font-mono text-lg tracking-widest text-slate-500"
    >
      {draftView.current_picker_name?.toUpperCase() ?? 'OPPONENT'}'S PICK
    </div>
  {/if}
</div>

<!-- Opponent rarity breakdown (bottom left) -->
{#if rarityEntries.length > 0}
  <div
    class="pointer-events-none absolute bottom-4 left-4 z-40 flex gap-2 rounded-lg border border-slate-600/30 bg-slate-950/80 px-3 py-2 font-mono text-xs text-slate-400"
  >
    <span class="text-slate-500">OPP:</span>
    {#each rarityEntries as [rarity, count]}
      <span class="capitalize">{rarity}: {count}</span>
    {/each}
  </div>
{/if}
