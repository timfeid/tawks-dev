<script lang="ts">
  import type { CardView } from "../types";
  import { hasRuleTermMatch, parseRulesText } from "./effect-glossary";
  import * as Tooltip from '@tawks/ui/tooltip/index.js';

  let {
    inspectedCard,
    hoveredCard,
    pointerPos,
    onHighlightCard = () => {},
  }: {
    inspectedCard: CardView | null;
    hoveredCard: CardView | null;
    pointerPos: { x: number; y: number };
    onHighlightCard?: (cardId: string | null) => void;
  } = $props();

  let card = $derived(inspectedCard ?? hoveredCard);
  let primaryRulesText = $derived(card?.description || card?.effect_text || "");
  let hasGlossaryTerms = $derived.by(
    () =>
      hasRuleTermMatch(primaryRulesText) ||
      (card?.abilities?.some((ability) =>
        hasRuleTermMatch(ability.effect_text),
      ) ??
        false),
  );

  let modifiers = $derived(
    card?.stat_modifiers?.filter(
      (m) =>
        m.power_mod !== 0 ||
        m.toughness_mod !== 0 ||
        (m.granted_keywords?.length ?? 0) > 0,
    ) ?? [],
  );

  const MANA_ORDER = [
    "neon",
    "glitch",
    "chrome",
    "slag",
    "bio",
    "colorless",
  ] as const;
  const MANA_LABELS: Record<(typeof MANA_ORDER)[number], string> = {
    neon: "Neon",
    glitch: "Glitch",
    chrome: "Chrome",
    slag: "Slag",
    bio: "Bio",
    colorless: "Colorless",
  };

  function formatMod(val: number): string {
    return val >= 0 ? `+${val}` : `${val}`;
  }

  function formatManaCost(
    cost: Record<string, number> | null | undefined,
  ): string {
    const parts = MANA_ORDER.flatMap((key) => {
      const amount = Number(cost?.[key] ?? 0);
      if (amount <= 0) return [];
      return amount === 1
        ? [MANA_LABELS[key]]
        : [`${amount} ${MANA_LABELS[key]}`];
    });
    return parts.length > 0 ? parts.join(", ") : "0";
  }
</script>

{#if card}
  {#snippet renderRuleText(text: string)}
    {@const segments = parseRulesText(text)}
    {#each segments as segment, index (`${segment.type}-${segment.text}-${index}`)}
      {#if segment.type === "term"}
        <Tooltip.Provider delayDuration={150}>
          <Tooltip.Root>
            <Tooltip.Trigger class="inline cursor-help border-b border-dotted border-cyan-400/50 text-cyan-50 transition-colors hover:border-cyan-300 hover:text-white">
              {segment.text}
            </Tooltip.Trigger>
            <Tooltip.Content class="z-[9999] w-64 rounded-lg border border-cyan-500/20 bg-slate-950 p-2 text-left not-italic shadow-2xl">
              <span class="block font-mono text-[10px] uppercase tracking-[0.14em] text-cyan-300">
                {segment.title}
              </span>
              <span class="mt-1 block text-[11px] leading-relaxed text-slate-200">
                {segment.description}
              </span>
            </Tooltip.Content>
          </Tooltip.Root>
        </Tooltip.Provider>
      {:else}
        <span>{segment.text}</span>
      {/if}
    {/each}
  {/snippet}

  <div
    class="pointer-events-none absolute right-8 bottom-96 z-10 flex flex-col items-end gap-3 transition-all duration-300"
  >
    <div
      class="pointer-events-auto w-80 rounded-2xl border border-white/20 bg-slate-950/90 p-4 text-slate-100 shadow-2xl backdrop-blur-md overflow-visible"
    >
      <div
        class="flex items-center justify-between border-b border-white/10 pb-2 mb-2"
      >
        <p
          class="font-mono text-[10px] tracking-[0.2em] text-cyan-400 uppercase"
        >
          Inspect
        </p>
        <p class="font-mono text-[10px] text-slate-500 uppercase">
          {card.kind_label}
        </p>
      </div>

      <div class="flex items-center justify-between gap-2">
        <h3 class="font-mono text-lg font-bold text-white leading-tight">
          {card.name}
        </h3>

        <!-- Rarity Badge -->
        <span
          class="rounded px-1.5 py-0.5 font-mono text-[9px] font-bold uppercase tracking-wider"
          style="
            background: {card.rarity === 'public'
            ? 'rgba(148, 163, 184, 0.2)'
            : card.rarity === 'private'
              ? 'rgba(56, 189, 248, 0.2)'
              : card.rarity === 'restricted'
                ? 'rgba(168, 85, 247, 0.2)'
                : card.rarity === 'classified'
                  ? 'rgba(244, 63, 94, 0.2)'
                  : 'rgba(250, 204, 21, 0.2)'};
            color: {card.rarity === 'public'
            ? '#94a3b8'
            : card.rarity === 'private'
              ? '#38bdf8'
              : card.rarity === 'restricted'
                ? '#a855f7'
                : card.rarity === 'classified'
                  ? '#f43f5e'
                  : '#facc15'};
            border: 1px solid currentColor;
          "
        >
          {card.rarity === "public"
            ? "Public"
            : card.rarity === "private"
              ? "Private"
              : card.rarity === "restricted"
                ? "Restricted"
                : card.rarity === "classified"
                  ? "Classified"
                  : "Singularity"}
        </span>
      </div>

      {#if card.categories && card.categories.length > 0}
        <div class="mt-1 flex flex-wrap gap-1">
          {#each card.categories as category}
            <span
              class="rounded-full bg-slate-800/50 px-2 py-0.5 font-mono text-[8px] text-slate-400 uppercase tracking-tighter"
            >
              {category.replace(/_/g, " ")}
            </span>
          {/each}
        </div>
      {/if}

      <p class="mt-2 font-mono text-xs text-slate-400">
        Cost: {formatManaCost(
          card.cost || {
            neon: 0,
            glitch: 0,
            chrome: 0,
            slag: 0,
            bio: 0,
            colorless: 0,
          },
        )}
      </p>

      {#if hasGlossaryTerms}
        <p
          class="mt-2 font-mono text-[10px] uppercase tracking-[0.14em] text-cyan-300/80"
        >
          Hover highlighted terms for definitions
        </p>
      {/if}

      {#if card.attached_to_name || card.remembered_player_name}
        <div class="mt-2 space-y-1 border-l border-cyan-500/20 pl-2">
          {#if card.attached_to_name}
            <p
              class="font-mono text-[10px] uppercase tracking-wider text-slate-400"
            >
              Attached To: {card.attached_to_name}
            </p>
          {/if}
          {#if card.remembered_player_name}
            <p
              class="font-mono text-[10px] uppercase tracking-wider text-cyan-300"
            >
              Chosen Player: {card.remembered_player_name}
            </p>
          {/if}
        </div>
      {/if}

      {#if card.description || card.effect_text}
        <div class="mt-3 rounded-lg bg-white/5 p-2 border border-white/5 overflow-visible">
          <div class="font-sans text-xs text-slate-200 leading-relaxed italic overflow-visible">
            {@render renderRuleText(card.description || card.effect_text || "")}
          </div>
        </div>
      {/if}

      {#if card.abilities && card.abilities.length > 0}
        <div class="mt-3 space-y-2">
          <p
            class="font-mono text-[9px] text-slate-500 uppercase tracking-widest"
          >
            Abilities
          </p>
          {#each card.abilities as ability}
            <div
              class="rounded p-2 border {ability.can_activate
                ? 'bg-cyan-500/10 border-cyan-500/20'
                : 'bg-white/5 border-white/10 opacity-60'}"
            >
              <div class="flex items-center justify-between">
                <span
                  class="font-mono text-xs font-bold {ability.can_activate
                    ? 'text-cyan-100'
                    : 'text-slate-400'}">{ability.name}</span
                >
                {#if formatManaCost(ability.cost?.mana) !== "0"}
                  <span
                    class="text-[10px] {ability.can_activate
                      ? 'text-cyan-400'
                      : 'text-slate-500'}"
                    >{formatManaCost(ability.cost?.mana)}</span
                  >
                {/if}
              </div>
              {#if ability.effect_text}
                <div class="mt-1 text-[11px] leading-relaxed text-slate-300">
                  {@render renderRuleText(ability.effect_text)}
                </div>
              {/if}
              {#if ability.inactive_reason}
                <p class="mt-1 font-mono text-[10px] text-amber-400/80">
                  {ability.inactive_reason}
                </p>
              {/if}
            </div>
          {/each}
        </div>
      {/if}

      {#if card.power !== undefined && card.power !== null}
        <div class="mt-4 border-t border-white/10 pt-2">
          <div class="flex items-center justify-between">
            <span class="font-mono text-[10px] text-slate-500 uppercase"
              >System Stats</span
            >
            <p class="font-mono text-sm font-bold text-cyan-200">
              {card.power} / {card.toughness}
            </p>
          </div>
          {#if modifiers.length > 0}
            <div class="mt-1.5 text-[10px] font-mono text-slate-500">
              Base: {card.base_power ?? card.power}/{card.base_toughness ??
                card.toughness}
            </div>
            <div class="pointer-events-auto mt-1 space-y-0.5">
              {#each modifiers as mod}
                <div
                  class="flex items-center justify-between rounded px-1.5 py-0.5 transition-colors duration-150 cursor-default hover:bg-white/10"
                  role="listitem"
                  onmouseenter={() =>
                    mod.source_card_id && onHighlightCard(mod.source_card_id)}
                  onmouseleave={() => onHighlightCard(null)}
                >
                  <span
                    class="font-mono text-[10px] text-slate-300 truncate mr-2"
                    >{mod.label}</span
                  >
                  <span class="flex items-center gap-1.5 whitespace-nowrap">
                    {#if mod.granted_keywords && mod.granted_keywords.length > 0}
                      <span class="font-mono text-[10px] font-bold text-cyan-300">
                        {mod.granted_keywords.join(', ')}
                      </span>
                    {/if}
                    {#if mod.power_mod !== 0 || mod.toughness_mod !== 0}
                      <span
                        class="font-mono text-[10px] font-bold {mod.power_mod >
                          0 || mod.toughness_mod > 0
                          ? 'text-green-400'
                          : 'text-red-400'}"
                      >
                        {formatMod(mod.power_mod)}/{formatMod(mod.toughness_mod)}
                      </span>
                    {/if}
                  </span>
                </div>
              {/each}
            </div>
          {/if}
        </div>
      {/if}

      {#if card.loyalty !== undefined && card.loyalty !== null}
        <div
          class="mt-4 flex items-center justify-between border-t border-white/10 pt-2"
        >
          <span class="font-mono text-[10px] text-slate-500 uppercase"
            >Core Loyalty</span
          >
          <p class="font-mono text-sm font-bold text-slate-300">
            {card.loyalty}
          </p>
        </div>
      {/if}
    </div>
  </div>
{/if}
