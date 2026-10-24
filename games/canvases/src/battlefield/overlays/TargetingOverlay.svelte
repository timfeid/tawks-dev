<script lang="ts">
  import type { CardView, CombatPairView } from '../types';

  type BlockPreview = {
    attacker: CardView;
    blocker: CardView;
    attackerDamage: number;
    blockerDamage: number;
    attackerDies: boolean;
    blockerDies: boolean;
    attackerProtectedByIndestructible: boolean;
    blockerProtectedByIndestructible: boolean;
    existingBlockers: number;
    rulesNote: string | null;
  };

  export let targetingSource: CardView | null;
  export let targetingMode: 'cast' | 'attack' | 'block' | 'activate' | null;
  export let targetingAbility: any = null;
  export let hoveredCard: CardView | null = null;
  export let combatPairs: CombatPairView[] = [];
  export let onCancel: () => void;
  // Screen-space position (px, relative to canvas top-left) of the source
  // card, so the dialog can float just above it in world space.
  export let targetingScreenPos: { x: number; y: number } | null = null;

  function activeTargetSpec() {
    if (targetingMode === 'activate' && targetingAbility?.target_spec) {
      return targetingAbility.target_spec;
    }
    return targetingSource?.target_spec ?? null;
  }

  function targetKindLabel() {
    const spec = activeTargetSpec();
    switch (spec?.kind) {
      case 'player':
        return 'player';
      case 'player_or_unit':
        return 'player or unit';
      case 'card':
        return 'card';
      case 'unit':
      default:
        return 'unit';
    }
  }

  function statValue(value: number | null | undefined): number {
    return Number(value ?? 0);
  }

  function totalDurability(card: CardView | null): number {
    if (!card) return 0;
    return (
      statValue(card.toughness) + statValue(card.armor) + statValue(card.shield)
    );
  }

  function hasKeyword(card: CardView | null, keyword: string): boolean {
    return (card?.keywords ?? []).some(
      (value) => String(value).toLowerCase() === keyword.toLowerCase(),
    );
  }

  function cardRulesText(card: CardView | null): string {
    return String(card?.description || card?.effect_text || '').toLowerCase();
  }

  function hasDamageTriggeredAbility(card: CardView | null): boolean {
    const text = cardRulesText(card);
    return /(when damaged|takes damage|whenever .* damaged|whenever .* takes damage)/.test(
      text,
    );
  }

  function buildRulesNote(card: CardView | null): string | null {
    if (!hasDamageTriggeredAbility(card)) return null;
    return 'Damage-triggered abilities resolve after combat damage. They do not save a unit from lethal damage already marked in combat.';
  }

  function buildBlockPreview(): BlockPreview | null {
    if (targetingMode !== 'block' || !targetingSource || !hoveredCard)
      return null;

    const pair = combatPairs.find(
      (entry) => entry.attacker.id === hoveredCard.id,
    );
    if (!pair) return null;

    const attacker = pair.attacker;
    const blocker = targetingSource;
    const attackerPower = statValue(attacker.power);
    const attackerHasDeathtouch = hasKeyword(attacker, 'Deathtouch');
    const attackerIndestructible = hasKeyword(attacker, 'Indestructible');
    const blockerIndestructible = hasKeyword(blocker, 'Indestructible');
    const existingBlockers = (pair.blockers || []).filter(
      (entry) => entry.id !== blocker.id,
    );

    let remainingDamage = attackerPower;
    let blockerDamage = 0;
    let attackerDamage = 0;
    let anyBlockerHasDeathtouch = false;

    for (const currentBlocker of [...existingBlockers, blocker]) {
      const blockerPower = statValue(currentBlocker.power);
      if (blockerPower > 0) {
        attackerDamage += blockerPower;
        if (hasKeyword(currentBlocker, 'Deathtouch')) {
          anyBlockerHasDeathtouch = true;
        }
      }

      const damageToAssign =
        remainingDamage <= 0
          ? 0
          : attackerHasDeathtouch
            ? 1
            : Math.min(remainingDamage, totalDurability(currentBlocker));

      if (currentBlocker.id === blocker.id) {
        blockerDamage = damageToAssign;
      }

      remainingDamage = Math.max(0, remainingDamage - damageToAssign);
    }

    const blockerDies =
      !blockerIndestructible &&
      ((attackerHasDeathtouch && blockerDamage > 0) ||
        blockerDamage >= totalDurability(blocker));
    const attackerDies =
      !attackerIndestructible &&
      ((anyBlockerHasDeathtouch && attackerDamage > 0) ||
        attackerDamage >= totalDurability(attacker));

    const rulesNote: string | null =
      (blockerDies && buildRulesNote(blocker)) ||
      (attackerDies && buildRulesNote(attacker)) ||
      null;

    return {
      attacker,
      blocker,
      attackerDamage,
      blockerDamage,
      attackerDies,
      blockerDies,
      attackerProtectedByIndestructible:
        attackerIndestructible && attackerDamage > 0,
      blockerProtectedByIndestructible:
        blockerIndestructible && blockerDamage > 0,
      existingBlockers: existingBlockers.length,
      rulesNote,
    };
  }

  function outcomeLabel(
    damage: number,
    dies: boolean,
    protectedByIndestructible: boolean,
    existingBlockers: number = 0,
  ): string {
    if (damage <= 0 && existingBlockers > 0) {
      return 'takes no damage because earlier blockers absorb the hit first.';
    }
    if (damage <= 0) {
      return 'takes no damage.';
    }
    if (protectedByIndestructible) {
      return `takes ${damage} and survives because it is Indestructible.`;
    }
    if (dies) {
      return `takes ${damage} and dies.`;
    }
    return `takes ${damage} and survives.`;
  }
</script>

{#if targetingSource}
  {@const floating = targetingScreenPos}
  <!-- Compact in-world targeting hint. Floats above the source card when
       its screen position is known; otherwise falls back to top-center. -->
  <div
    class="pointer-events-none absolute z-30 flex flex-col items-center gap-2 {floating
      ? '-translate-x-1/2 -translate-y-full'
      : 'inset-x-0 top-4 justify-center'}"
    style={floating
      ? `left: ${floating.x}px; top: ${Math.max(40, floating.y - 8)}px;`
      : ''}
  >
    <div
      class="pointer-events-auto flex items-center gap-3 rounded-full border border-cyan-400/30 bg-black/80 px-4 py-1.5 text-xs text-white shadow-lg backdrop-blur-sm"
    >
      <span class="font-mono uppercase tracking-[0.18em] text-cyan-300/80">
        {#if targetingMode === 'attack'}
          Attack
        {:else if targetingMode === 'block'}
          Block
        {:else if targetingMode === 'activate'}
          {targetingAbility?.name || 'Ability'}
        {:else}
          Cast
        {/if}
      </span>
      <span class="text-slate-400">·</span>
      <span class="text-cyan-200">
        {#if targetingMode === 'block'}
          pick attacker
        {:else if targetingMode === 'attack'}
          pick player or unit
        {:else}
          pick {targetKindLabel()}
        {/if}
      </span>
      <button
        class="ml-1 rounded-full border border-white/10 bg-white/5 px-2 py-0.5 text-[10px] font-bold uppercase tracking-wider text-slate-200 transition hover:border-rose-400/50 hover:bg-rose-500/20 hover:text-white"
        onclick={onCancel}
        aria-label="Cancel targeting"
      >
        Esc / RMB
      </button>
    </div>

    {#if targetingMode === 'block'}
      {@const preview = buildBlockPreview()}
      {#if preview}
        <div
          class="pointer-events-auto w-[32rem] max-w-[90vw] rounded-xl border border-amber-400/20 bg-amber-500/10 px-4 py-3 shadow-2xl"
        >
          <div
            class="text-center text-[10px] font-semibold uppercase tracking-[0.24em] text-amber-200"
          >
            Combat Preview
          </div>
          <div class="mt-2 grid gap-2 sm:grid-cols-2">
            <div
              class="rounded-lg border border-white/10 bg-black/30 px-3 py-2"
            >
              <div
                class="flex items-center justify-between gap-3 text-sm font-semibold text-white"
              >
                <span class="truncate">{preview.blocker.name}</span>
                <span class="font-mono text-xs text-cyan-200">
                  {statValue(preview.blocker.power)}/{statValue(
                    preview.blocker.toughness,
                  )}
                </span>
              </div>
              <div
                class="mt-1 text-xs {preview.blockerDies
                  ? 'text-rose-200'
                  : 'text-emerald-200'}"
              >
                {outcomeLabel(
                  preview.blockerDamage,
                  preview.blockerDies,
                  preview.blockerProtectedByIndestructible,
                  preview.existingBlockers,
                )}
              </div>
            </div>
            <div
              class="rounded-lg border border-white/10 bg-black/30 px-3 py-2"
            >
              <div
                class="flex items-center justify-between gap-3 text-sm font-semibold text-white"
              >
                <span class="truncate">{preview.attacker.name}</span>
                <span class="font-mono text-xs text-cyan-200">
                  {statValue(preview.attacker.power)}/{statValue(
                    preview.attacker.toughness,
                  )}
                </span>
              </div>
              <div
                class="mt-1 text-xs {preview.attackerDies
                  ? 'text-rose-200'
                  : 'text-emerald-200'}"
              >
                {outcomeLabel(
                  preview.attackerDamage,
                  preview.attackerDies,
                  preview.attackerProtectedByIndestructible,
                )}
              </div>
            </div>
          </div>
          {#if preview.existingBlockers > 0}
            <div
              class="mt-2 rounded-lg border border-white/10 bg-black/20 px-3 py-2 text-xs text-slate-200"
            >
              Damage is assigned in blocker order. {preview.blocker.name} is checked
              after {preview.existingBlockers} existing blocker{preview.existingBlockers ===
              1
                ? ''
                : 's'}.
            </div>
          {/if}
          {#if preview.rulesNote}
            <div
              class="mt-2 rounded-lg border border-rose-400/20 bg-rose-500/10 px-3 py-2 text-xs text-rose-100"
            >
              {preview.rulesNote}
            </div>
          {/if}
        </div>
      {/if}
    {/if}
  </div>
{/if}
