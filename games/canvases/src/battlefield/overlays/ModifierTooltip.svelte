<script lang="ts">
  import type { CardView } from '../types';
  import type { ActionHint } from '../engine/InteractionController';

  let {
    card,
    actionHints,
    pointerPos,
    onHighlightCard,
  }: {
    card: CardView | null;
    actionHints?: ActionHint[] | null;
    pointerPos: { x: number; y: number };
    onHighlightCard: (cardId: string | null) => void;
  } = $props();

  let modifiers = $derived(card?.stat_modifiers?.filter(
    (m) =>
      m.power_mod !== 0 ||
      m.toughness_mod !== 0 ||
      (m.granted_keywords?.length ?? 0) > 0,
  ) ?? []);

  let showMods = $derived(
    card != null &&
    card.power != null &&
    card.toughness != null &&
    modifiers.length > 0,
  );

  function formatMod(val: number): string {
    return val >= 0 ? `+${val}` : `${val}`;
  }
</script>

{#if card}
  <div
    class="pointer-events-none absolute z-20 flex flex-col gap-2"
    style="left: {pointerPos.x + 20}px; top: {pointerPos.y - 40}px;"
  >
    {#if showMods}
      <div class="pointer-events-auto w-56 rounded-xl border border-white/15 bg-slate-950/92 p-3 text-slate-100 shadow-2xl backdrop-blur-md">
        <div class="flex items-center justify-between mb-2">
          <span class="font-mono text-[9px] tracking-[0.15em] text-cyan-400/80 uppercase">Modifiers</span>
          <span class="font-mono text-xs font-bold text-white">
            {card.base_power ?? card.power}/{card.base_toughness ?? card.toughness}
            <span class="text-slate-500 mx-0.5">&rarr;</span>
            <span class="{(card.power ?? 0) > (card.base_power ?? card.power ?? 0) ? 'text-green-400' : (card.power ?? 0) < (card.base_power ?? card.power ?? 0) ? 'text-red-400' : ''}">
              {card.power}/{card.toughness}
            </span>
          </span>
        </div>
        <div class="space-y-1">
          {#each modifiers as mod}
            <div
              class="flex items-center justify-between rounded px-1.5 py-0.5 text-xs transition-colors duration-150 cursor-default hover:bg-white/10"
              role="listitem"
              onmouseenter={() => mod.source_card_id && onHighlightCard(mod.source_card_id)}
              onmouseleave={() => onHighlightCard(null)}
            >
              <span class="font-mono text-slate-300 truncate mr-2">{mod.label}</span>
              <span class="flex items-center gap-1.5 whitespace-nowrap">
                {#if mod.granted_keywords && mod.granted_keywords.length > 0}
                  <span class="font-mono font-bold text-cyan-300">
                    {mod.granted_keywords.join(', ')}
                  </span>
                {/if}
                {#if mod.power_mod !== 0 || mod.toughness_mod !== 0}
                  <span class="font-mono font-bold {mod.power_mod > 0 || mod.toughness_mod > 0 ? 'text-green-400' : 'text-red-400'}">
                    {formatMod(mod.power_mod)}/{formatMod(mod.toughness_mod)}
                  </span>
                {/if}
              </span>
            </div>
          {/each}
        </div>
      </div>
    {/if}
    
    <div class="w-max rounded-lg border border-white/10 bg-slate-950/90 px-3 py-2 text-slate-300 shadow-xl backdrop-blur-md flex flex-col gap-1.5 pointer-events-none">
      {#if actionHints}
        {#each actionHints as hint}
          <div class="flex items-center gap-2.5 text-[10px] font-mono uppercase tracking-widest {hint.disabled ? 'opacity-40' : ''}">
            <div class="flex items-center justify-center rounded bg-white/10 border border-white/20 px-1.5 h-5 text-white leading-none shadow-[0_0_8px_rgba(255,255,255,0.1)]">
              {hint.key}
            </div>
            <span class="text-amber-400/90 font-semibold">{hint.label}</span>
          </div>
        {/each}
      {/if}
      <div class="flex items-center gap-2.5 text-[10px] font-mono uppercase tracking-widest">
        <div class="flex items-center justify-center rounded bg-white/10 border border-white/20 px-1.5 h-5 text-white leading-none shadow-[0_0_8px_rgba(255,255,255,0.1)]">
          RMB
        </div>
        <span class="text-slate-500 lowercase px-0.5">or</span>
        <div class="flex items-center justify-center rounded bg-white/10 border border-white/20 px-1.5 h-5 text-white leading-none shadow-[0_0_8px_rgba(255,255,255,0.1)]">
          F
        </div>
        <span class="text-cyan-400/90 font-semibold">Inspect Card</span>
      </div>
    </div>
  </div>
{/if}

