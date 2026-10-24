<script lang="ts">
  import type { CardAction } from '../actions';

  export let card: any;
  export let actions: CardAction[] = [];
  export let x: number;
  export let y: number;
  export let onSelect: (action: CardAction) => void;
  export let onClose: () => void;
</script>

{#if card && actions.length > 0}
  <div class="absolute inset-0 z-50" onclick={onClose}>
    <div
      class="pointer-events-auto absolute w-56 rounded-xl border border-cyan-500/30 bg-slate-950/95 p-3 shadow-[0_0_30px_rgba(6,182,212,0.15)]"
      style="left: {x}px; top: {y}px; transform: translate(-50%, 0);"
      onclick={(e) => e.stopPropagation()}
    >
      <div class="mb-2 text-[11px] font-mono uppercase tracking-widest text-cyan-300">
        Actions
      </div>
      <div class="mb-3 text-sm font-bold text-white">
        {card.name}
      </div>
      <div class="flex flex-col gap-2">
        {#each actions as action}
          {#if action.disabled}
            <div
              class="w-full rounded-lg border border-white/5 bg-white/[0.02] px-3 py-2 text-left opacity-50 cursor-not-allowed"
            >
              <span class="text-sm text-slate-400">{action.label}</span>
              {#if action.disabledReason}
                <span class="ml-2 text-[10px] text-amber-400/70">{action.disabledReason}</span>
              {/if}
            </div>
          {:else}
            <button
              class="w-full rounded-lg border border-white/10 bg-white/5 px-3 py-2 text-left text-sm text-slate-100 transition hover:border-cyan-400/60 hover:bg-cyan-500/10"
              onclick={() => onSelect(action)}
            >
              {action.label}
            </button>
          {/if}
        {/each}
      </div>
      <button
        class="mt-3 w-full rounded-lg border border-white/10 bg-white/5 px-3 py-1.5 text-xs uppercase tracking-widest text-slate-400 transition hover:border-slate-300/60 hover:text-slate-200"
        onclick={onClose}
      >
        Cancel
      </button>
    </div>
  </div>
{/if}
