<script lang="ts">
  import type { CardView, DraftAwardsView } from '../types';

  let {
    awardsView,
    selectedCount,
    focusedCard,
    selectedIds,
    onToggleFocused,
    onSubmit,
  }: {
    awardsView: DraftAwardsView;
    selectedCount: number;
    focusedCard: CardView | null;
    selectedIds: Set<string>;
    onToggleFocused: () => void;
    onSubmit: () => void;
  } = $props();

  let waitingLabel = $derived((awardsView.waiting_on || []).join(', '));
  let actionLabel = $derived('Claim Selected');
  let canSubmit = $derived(
    !awardsView.submitted &&
      awardsView.selection_mode === 'claim' &&
      selectedCount === awardsView.required_count,
  );
</script>

<div class="pointer-events-none absolute inset-x-0 top-4 z-40 flex justify-center px-4">
  <div class="w-full max-w-3xl rounded-3xl border border-amber-400/20 bg-black/75 p-5 text-slate-100 shadow-2xl shadow-amber-950/30 backdrop-blur-xl">
    <div class="flex items-start justify-between gap-4">
      <div>
        <div class="text-[10px] uppercase tracking-[0.22em] text-amber-300/80">Match Results</div>
        <div class="mt-1 text-2xl font-semibold text-white">
          {awardsView.winner_name ?? 'Winner'} Wins
        </div>
        {#if awardsView.role === 'winner'}
          <div class="mt-2 text-sm text-slate-300">
            Pick {awardsView.required_count} cards to claim. The rest go to the loser.
          </div>
        {:else if awardsView.role === 'loser'}
          <div class="mt-2 text-sm text-slate-300">
            Waiting for the winner to make their picks. You'll get the rest.
          </div>
        {:else}
          <div class="mt-2 text-sm text-slate-300">
            Awards are being resolved.
          </div>
        {/if}
      </div>
      <div class="rounded-2xl border border-white/10 bg-white/5 px-4 py-3 text-right">
        <div class="text-[10px] uppercase tracking-[0.18em] text-slate-500">Selected</div>
        <div class="mt-1 text-2xl font-semibold text-amber-200">{selectedCount}</div>
        {#if awardsView.required_count > 0}
          <div class="text-xs text-slate-400">of {awardsView.required_count}</div>
        {/if}
      </div>
    </div>

    {#if awardsView.submitted}
      <div class="mt-4 rounded-2xl border border-emerald-400/20 bg-emerald-500/10 px-4 py-3 text-sm text-emerald-200">
        Awards submitted.
      </div>
    {:else if awardsView.selection_mode === 'none' && waitingLabel}
      <div class="mt-4 rounded-2xl border border-slate-700/60 bg-slate-900/70 px-4 py-3 text-sm text-slate-300">
        Waiting on {waitingLabel}.
      </div>
    {:else}
      <div class="mt-4 flex items-center gap-3">
        <button
          class="pointer-events-auto rounded-2xl border border-slate-600/70 bg-slate-900/80 px-4 py-2 text-sm font-medium text-slate-200 transition hover:border-amber-400/50 hover:text-amber-100"
          onclick={onToggleFocused}
          disabled={!focusedCard}
        >
          {#if focusedCard && selectedIds.has(focusedCard.id)}
            Unselect Focused Card
          {:else}
            Select Focused Card
          {/if}
        </button>
        <button
          class="pointer-events-auto rounded-2xl border border-amber-400/40 bg-amber-500/15 px-4 py-2 text-sm font-semibold text-amber-100 transition enabled:hover:bg-amber-500/25 disabled:cursor-not-allowed disabled:border-slate-700/60 disabled:bg-slate-900/70 disabled:text-slate-500"
          onclick={onSubmit}
          disabled={!canSubmit}
        >
          {actionLabel}
        </button>
        {#if focusedCard}
          <div class="text-sm text-slate-400">
            Focused: <span class="text-slate-200">{focusedCard.name}</span>
          </div>
        {/if}
      </div>
    {/if}
  </div>
</div>
