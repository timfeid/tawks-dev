<script lang="ts">
  import { submitServerAction } from '@tawks/workflow-ui/components/workflow';
  import { getContext } from 'svelte';
  import type { WorkflowStateProvider } from '@tawks/workflow-ui/workflow';

  let {
    workflowId,
    stage,
    combatPhase,
    stackLength,
    show,
    isActive,
    isCurrentBlocker,
    stackTop = null,
    viewerId = null,
    hide = false,
    pendingDiscard = null,
    discardSelectedCount = 0,
    onDiscardCommit,
  }: {
    workflowId: string;
    stage: string | undefined;
    combatPhase: string | undefined;
    stackLength: number;
    show: boolean;
    isActive: boolean;
    isCurrentBlocker: boolean;
    stackTop?: any;
    viewerId?: string | null;
    hide?: boolean;
    pendingDiscard?: { amount: number } | null;
    discardSelectedCount?: number;
    onDiscardCommit?: () => void;
  } = $props();

  const provider = getContext<WorkflowStateProvider>('workflow-provider');
</script>

{#if show && !hide}
  <div class="absolute right-8 bottom-40 z-50 flex flex-col items-end gap-4">
    {#if pendingDiscard}
      {@const ready = discardSelectedCount === pendingDiscard.amount}
      <button
        class="group relative flex h-16 w-60 items-center justify-center overflow-hidden rounded-xl border font-mono text-lg font-bold tracking-widest text-white transition-all active:scale-95 {ready
          ? 'border-purple-500/50 bg-slate-950/90 shadow-[0_0_20px_rgba(168,85,247,0.25)] hover:scale-105 hover:border-purple-400 hover:shadow-[0_0_30px_rgba(168,85,247,0.45)]'
          : 'cursor-not-allowed border-purple-500/20 bg-slate-950/60 text-white/40 shadow-none'}"
        disabled={!ready}
        onclick={() => onDiscardCommit?.()}
      >
        <div
          class="absolute inset-0 translate-x-[-100%] bg-gradient-to-r from-transparent via-purple-500/10 to-transparent transition-transform duration-1000 group-hover:translate-x-[100%]"
        ></div>
        <span class="relative z-10 flex items-center gap-3">
          DISCARD
          <kbd class="rounded border border-white/20 bg-white/10 px-1.5 py-0.5 text-[10px] font-bold text-white/60">SPACE</kbd>
        </span>
      </button>
    {:else if stackLength > 0 && stackTop?.owner_id !== viewerId}
      <button
        class="group relative flex h-16 w-60 items-center justify-center overflow-hidden rounded-xl border border-amber-500/50 bg-slate-950/90 font-mono text-lg font-bold tracking-widest text-white shadow-[0_0_20px_rgba(245,158,11,0.2)] transition-all hover:scale-105 hover:border-amber-400 hover:shadow-[0_0_30px_rgba(245,158,11,0.4)] active:scale-95"
        onclick={() => submitServerAction(provider, workflowId, {}, 'games.cardgame.pass_priority')}
      >
        <div
          class="absolute inset-0 translate-x-[-100%] bg-gradient-to-r from-transparent via-amber-500/10 to-transparent transition-transform duration-1000 group-hover:translate-x-[100%]"
        ></div>
        <span class="relative z-10 flex items-center gap-3">
          PASS
          <kbd class="rounded border border-white/20 bg-white/10 px-1.5 py-0.5 text-[10px] font-bold text-white/60">SPACE</kbd>
        </span>
      </button>
    {:else if stage === 'main' && stackLength === 0}
      <button
        class="group relative flex h-16 w-60 items-center justify-center overflow-hidden rounded-xl border border-red-500/50 bg-slate-950/90 font-mono text-lg font-bold tracking-widest text-white shadow-[0_0_20px_rgba(239,68,68,0.2)] transition-all hover:scale-105 hover:border-red-400 hover:shadow-[0_0_30px_rgba(239,68,68,0.4)] active:scale-95"
        onclick={() => submitServerAction(provider, workflowId, {}, 'games.cardgame.next')}
      >
        <div
          class="absolute inset-0 translate-x-[-100%] bg-gradient-to-r from-transparent via-red-500/10 to-transparent transition-transform duration-1000 group-hover:translate-x-[100%]"
        ></div>
        <span class="relative z-10 flex items-center gap-3">
          COMBAT
          <kbd class="rounded border border-white/20 bg-white/10 px-1.5 py-0.5 text-[10px] font-bold text-white/60">SPACE</kbd>
        </span>
      </button>
    {:else if stage === 'combat' && combatPhase === 'declare_attackers' && isActive}
      <button
        class="group relative flex h-16 w-72 items-center justify-center overflow-hidden rounded-xl border border-orange-500/50 bg-slate-950/90 font-mono text-base font-bold tracking-widest text-white shadow-[0_0_20px_rgba(249,115,22,0.2)] transition-all hover:scale-105 hover:border-orange-400 hover:shadow-[0_0_30px_rgba(249,115,22,0.4)] active:scale-95"
        onclick={() => submitServerAction(provider, workflowId, {}, 'games.cardgame.confirm_attackers')}
      >
        <div
          class="absolute inset-0 translate-x-[-100%] bg-gradient-to-r from-transparent via-orange-500/10 to-transparent transition-transform duration-1000 group-hover:translate-x-[100%]"
        ></div>
        <span class="relative z-10 flex items-center gap-3">
          DECLARE ATTACKERS
          <kbd class="rounded border border-white/20 bg-white/10 px-1.5 py-0.5 text-[10px] font-bold text-white/60">SPACE</kbd>
        </span>
      </button>
    {:else if stage === 'combat' && combatPhase === 'declare_blockers' && isCurrentBlocker}
      <button
        class="group relative flex h-16 w-72 items-center justify-center overflow-hidden rounded-xl border border-blue-500/50 bg-slate-950/90 font-mono text-base font-bold tracking-widest text-white shadow-[0_0_20px_rgba(59,130,246,0.2)] transition-all hover:scale-105 hover:border-blue-400 hover:shadow-[0_0_30px_rgba(59,130,246,0.4)] active:scale-95"
        onclick={() => submitServerAction(provider, workflowId, {}, 'games.cardgame.confirm_blockers')}
      >
        <div
          class="absolute inset-0 translate-x-[-100%] bg-gradient-to-r from-transparent via-blue-500/10 to-transparent transition-transform duration-1000 group-hover:translate-x-[100%]"
        ></div>
        <span class="relative z-10 flex items-center gap-3">
          DECLARE BLOCKERS
          <kbd class="rounded border border-white/20 bg-white/10 px-1.5 py-0.5 text-[10px] font-bold text-white/60">SPACE</kbd>
        </span>
      </button>
    {:else if stage === 'post_combat_main'}
      <button
        class="group relative flex h-16 w-60 items-center justify-center overflow-hidden rounded-xl border border-red-500/50 bg-slate-950/90 font-mono text-lg font-bold tracking-widest text-white shadow-[0_0_20px_rgba(239,68,68,0.2)] transition-all hover:scale-105 hover:border-red-400 hover:shadow-[0_0_30px_rgba(239,68,68,0.4)] active:scale-95"
        onclick={() => submitServerAction(provider, workflowId, {}, 'games.cardgame.next')}
      >
        <div
          class="absolute inset-0 translate-x-[-100%] bg-gradient-to-r from-transparent via-red-500/10 to-transparent transition-transform duration-1000 group-hover:translate-x-[100%]"
        ></div>
        <span class="relative z-10 flex items-center gap-3">
          END TURN
          <kbd class="rounded border border-white/20 bg-white/10 px-1.5 py-0.5 text-[10px] font-bold text-white/60">SPACE</kbd>
        </span>
      </button>
    {:else if stage === 'end'}
      <button
        class="group relative flex h-16 w-60 items-center justify-center overflow-hidden rounded-xl border border-emerald-500/50 bg-slate-950/90 font-mono text-lg font-bold tracking-widest text-white shadow-[0_0_20px_rgba(16,185,129,0.2)] transition-all hover:scale-105 hover:border-emerald-400 hover:shadow-[0_0_30px_rgba(16,185,129,0.4)] active:scale-95"
        onclick={() => submitServerAction(provider, workflowId, {}, 'games.cardgame.next')}
      >
        <div
          class="absolute inset-0 translate-x-[-100%] bg-gradient-to-r from-transparent via-emerald-500/10 to-transparent transition-transform duration-1000 group-hover:translate-x-[100%]"
        ></div>
        <span class="relative z-10 flex items-center gap-3">
          CONFIRM END
          <kbd class="rounded border border-white/20 bg-white/10 px-1.5 py-0.5 text-[10px] font-bold text-white/60">SPACE</kbd>
        </span>
      </button>
    {/if}
  </div>
{/if}
