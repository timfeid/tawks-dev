<script lang="ts">
  import type { CardView } from '../types';

  // Mirror the shape of DeckCardView (card + quantity) from the Rust view.
  type DeckCardEntry = {
    card: CardView;
    quantity: number;
  };

  let {
    sideboard = [],
    deckId = null,
    sideboardMode = false,
    onToggleMode,
    onAdd,
    onRemove,
  }: {
    sideboard: DeckCardEntry[];
    deckId: string | null;
    /** When true, clicks in the library go to sideboard instead of main. */
    sideboardMode: boolean;
    onToggleMode: (active: boolean) => void;
    onAdd: (cardId: string) => void;
    onRemove: (cardId: string) => void;
  } = $props();

  const total = $derived(sideboard.reduce((sum, e) => sum + (e.quantity || 0), 0));
  const MAX = 15;
  const atLimit = $derived(total >= MAX);
</script>

<div class="pointer-events-auto fixed right-4 top-20 z-40 flex w-[260px] flex-col rounded-lg border border-amber-500/40 bg-slate-950/90 shadow-[0_0_24px_rgba(251,191,36,0.2)] backdrop-blur-md">
  <!-- Header -->
  <div class="flex items-center justify-between border-b border-white/10 px-3 py-2">
    <div>
      <div class="font-mono text-[10px] uppercase tracking-widest text-amber-400">
        Sideboard
      </div>
      <div class="text-sm font-bold text-white">
        {total} / {MAX}
      </div>
    </div>
    <button
      class="flex items-center gap-1.5 rounded border px-2 py-1 text-[10px] font-mono uppercase tracking-wider transition"
      class:border-amber-400={sideboardMode}
      class:bg-amber-500={sideboardMode}
      class:text-slate-900={sideboardMode}
      class:border-slate-600={!sideboardMode}
      class:text-slate-300={!sideboardMode}
      class:opacity-50={atLimit && !sideboardMode}
      class:cursor-not-allowed={atLimit && !sideboardMode}
      disabled={atLimit && !sideboardMode}
      onclick={() => onToggleMode(!sideboardMode)}
      title={sideboardMode
        ? 'Clicks in the library add to sideboard. Click to return to main.'
        : atLimit
          ? 'Sideboard full'
          : 'Switch library clicks to add to sideboard'}
    >
      <span class="h-1.5 w-1.5 rounded-full" class:bg-slate-900={sideboardMode} class:bg-slate-500={!sideboardMode}></span>
      {sideboardMode ? 'Adding to side' : 'Add to side'}
    </button>
  </div>

  <!-- List -->
  <div class="max-h-[50vh] overflow-y-auto px-2 py-2">
    {#if sideboard.length === 0}
      <div class="px-2 py-6 text-center text-xs font-mono text-slate-500">
        Empty. Toggle "Add to side" and click library cards.
      </div>
    {:else}
      <div class="flex flex-col gap-1">
        {#each sideboard as entry (entry.card.id)}
          <div class="group flex items-center gap-2 rounded border border-slate-800 bg-slate-900 px-2 py-1.5 text-xs text-slate-200 transition hover:border-slate-600">
            <span class="flex-1 truncate">{entry.card.name}</span>
            <span class="font-mono text-slate-500">×{entry.quantity}</span>
            <div class="flex gap-1 opacity-60 transition group-hover:opacity-100">
              <button
                class="flex h-5 w-5 items-center justify-center rounded border border-slate-700 text-[11px] transition hover:border-red-500 hover:text-red-400"
                onclick={() => onRemove(entry.card.id)}
                title="Remove one"
                disabled={!deckId}
              >
                −
              </button>
              <button
                class="flex h-5 w-5 items-center justify-center rounded border border-slate-700 text-[11px] transition hover:border-green-500 hover:text-green-400"
                onclick={() => onAdd(entry.card.id)}
                title="Add one more"
                disabled={!deckId || atLimit}
              >
                +
              </button>
            </div>
          </div>
        {/each}
      </div>
    {/if}
  </div>
</div>
