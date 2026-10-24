<script lang="ts">
  import type { PlayerState, Ruleset } from '../types';

  let {
    players = [],
    viewerId = null,
    ruleset = 'mana_clash',
    onOpenDiscard = () => {},
    onTargetPlayer = () => false,
    canTargetPlayer = () => false,
  }: {
    players: PlayerState[];
    viewerId: string | null;
    ruleset?: Ruleset;
    onOpenDiscard?: (playerId: string) => void;
    /** Attempt to target the given player. Returns true if consumed. */
    onTargetPlayer?: (playerId: string) => boolean;
    /** Should the given player read as a clickable target right now? */
    canTargetPlayer?: (playerId: string) => boolean;
  } = $props();

  let opponents = $derived(players.filter((p) => p.id !== viewerId));
  let viewer = $derived(players.find((p) => p.id === viewerId));

  // Pulse the life total when it changes so the player registers the swing.
  const previousLife = new Map<string, number>();
  let lifePulse = $state<Record<string, 'damage' | 'gain' | null>>({});
  const pulseTimers = new Map<string, ReturnType<typeof setTimeout>>();

  $effect(() => {
    for (const p of players) {
      const prev = previousLife.get(p.id);
      if (prev !== undefined && prev !== p.life) {
        const direction: 'damage' | 'gain' = p.life < prev ? 'damage' : 'gain';
        lifePulse = { ...lifePulse, [p.id]: direction };
        const existing = pulseTimers.get(p.id);
        if (existing) clearTimeout(existing);
        const tid = setTimeout(() => {
          lifePulse = { ...lifePulse, [p.id]: null };
          pulseTimers.delete(p.id);
        }, 600);
        pulseTimers.set(p.id, tid);
      }
      previousLife.set(p.id, p.life);
    }
  });

  // Pulse the operator loyalty when it drops (visual feedback for combat
  // damage to operators — the "loyalty drain" the plan calls for).
  const previousLoyalty = new Map<string, number>();
  let loyaltyPulse = $state<Record<string, 'drain' | null>>({});
  const loyaltyPulseTimers = new Map<string, ReturnType<typeof setTimeout>>();

  $effect(() => {
    for (const p of players) {
      const op = p.operator_zone?.[0];
      if (!op || op.loyalty == null) continue;
      const key = `${p.id}:${op.id}`;
      const prev = previousLoyalty.get(key);
      if (prev !== undefined && op.loyalty < prev) {
        loyaltyPulse = { ...loyaltyPulse, [p.id]: 'drain' };
        const existing = loyaltyPulseTimers.get(p.id);
        if (existing) clearTimeout(existing);
        const tid = setTimeout(() => {
          loyaltyPulse = { ...loyaltyPulse, [p.id]: null };
          loyaltyPulseTimers.delete(p.id);
        }, 700);
        loyaltyPulseTimers.set(p.id, tid);
      }
      previousLoyalty.set(key, op.loyalty);
    }
  });

  const manaColors: Record<string, string> = {
    neon: '#00e5ff',
    glitch: '#a855f7',
    chrome: '#cbd5e1',
    slag: '#f97316',
    bio: '#22c55e',
    colorless: '#94a3b8'
  };

  type OpDamageBadge = { attackerId: string; attackerName: string; dmg: number; tier: 'low' | 'mid' | 'high' };

  function getOperatorDamageBadges(target: PlayerState): OpDamageBadge[] {
    const damage = target.operator_damage;
    if (!damage) return [];
    const out: OpDamageBadge[] = [];
    for (const [attackerId, dmg] of Object.entries(damage)) {
      if (!dmg || dmg <= 0) continue;
      const attackerOp = players.find((p) => p.id === attackerId)?.operator_zone?.[0];
      const attackerName = attackerOp?.name ?? '?';
      const tier = dmg >= 15 ? 'high' : dmg >= 10 ? 'mid' : 'low';
      out.push({ attackerId, attackerName, dmg, tier });
    }
    out.sort((a, b) => b.dmg - a.dmg);
    return out;
  }

  function getPlayerColor(p: PlayerState) {
    if (p.deck_colors && p.deck_colors.length > 0) {
      const colorKey = p.deck_colors[0].toLowerCase();
      if (manaColors[colorKey]) return manaColors[colorKey];
      return p.deck_colors[0];
    }
    if (p.operator_zone && p.operator_zone.length > 0 && p.operator_zone[0].colors?.length) {
      return p.operator_zone[0].colors[0];
    }
    if (p.battlefield_infrastructure) {
      for (const card of p.battlefield_infrastructure) {
        if (card.colors && card.colors.length > 0) return card.colors[0];
      }
    }
    if (p.grid && p.grid.length > 0) {
      const g = p.grid[0];
      if (g.color) return g.color;
      if (g.mana_type && manaColors[g.mana_type]) return manaColors[g.mana_type];
    }
    if (p.battlefield_units) {
      for (const card of p.battlefield_units) {
        if (card.colors && card.colors.length > 0) return card.colors[0];
      }
    }
    if (p.hand) {
      // The hand in PlayerState isn't actually passed to opponents, but viewer might have it
      // Actually `viewer` has `hand`. Opponents might not.
      const hand = (p as any).hand;
      if (hand) {
        for (const card of hand) {
          if (card.colors && card.colors.length > 0) return card.colors[0];
        }
      }
    }
    return '#22d3ee';
  }

  const HEAT_LINE = 10;

  function cashOf(p: PlayerState) {
    return p.mana_pool?.colorless ?? 0;
  }

  function heatTone(heat: number) {
    if (heat >= HEAT_LINE) return 'text-red-400';
    if (heat >= HEAT_LINE - 3) return 'text-amber-400';
    return 'text-slate-400';
  }

  function getMaxLife(p: PlayerState) {
    return p.operator_zone && p.operator_zone.length > 0 ? 40 : 20;
  }

  function getManaBlocks(p: PlayerState) {
    const blocks: { color: string, active: boolean, id: string }[] = [];
    let idCounter = 0;
    
    if (p.mana_pool) {
      for (const [manaType, count] of Object.entries(p.mana_pool)) {
        if (count > 0 && manaColors[manaType]) {
          for (let i = 0; i < count; i++) {
            blocks.push({ color: manaType, active: true, id: `active_${manaType}_${idCounter++}` });
          }
        }
      }
    }
    
    if (p.battlefield_infrastructure) {
      for (const infra of p.battlefield_infrastructure) {
        if (infra.tapped) {
          let manaType = 'colorless';
          if (infra.colors && infra.colors.length > 0) {
            const hex = infra.colors[0].toLowerCase();
            const entry = Object.entries(manaColors).find(([_, c]) => c.toLowerCase() === hex);
            if (entry) manaType = entry[0];
          } else {
            const name = infra.name.toLowerCase();
            if (name.includes('neon')) manaType = 'neon';
            else if (name.includes('glitch')) manaType = 'glitch';
            else if (name.includes('chrome')) manaType = 'chrome';
            else if (name.includes('slag')) manaType = 'slag';
            else if (name.includes('bio')) manaType = 'bio';
          }
          blocks.push({ color: manaType, active: false, id: `tapped_${manaType}_${idCounter++}` });
        }
      }
    }

    blocks.sort((a, b) => {
      if (a.color !== b.color) return a.color.localeCompare(b.color);
      return (a.active === b.active) ? 0 : a.active ? -1 : 1;
    });
    return blocks;
  }
</script>

<div class="pointer-events-none absolute inset-0 z-30 overflow-hidden">
  <!-- Top: Opponents HUD -->
  <div class="absolute top-4 left-0 w-full flex items-start justify-center gap-4 px-4">
    {#each opponents as opp (opp.id)}
      {@const deckColor = getPlayerColor(opp)}
      {@const maxLife = getMaxLife(opp)}
      {@const lifePct = Math.max(0, Math.min(100, (opp.life / maxLife) * 100))}
      {@const manaBlocks = getManaBlocks(opp)}
      {@const opDmgBadges = getOperatorDamageBadges(opp)}
      {@const oppOp = opp.operator_zone?.[0]}
      {@const canTarget = canTargetPlayer(opp.id)}
      <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
      <div
        role={canTarget ? 'button' : undefined}
        tabindex={canTarget ? 0 : undefined}
        class="flex flex-col bg-slate-950/80 backdrop-blur-md border border-white/5 shadow-lg transition-all rounded-md overflow-hidden min-w-[260px]"
        class:pointer-events-auto={canTarget}
        class:cursor-crosshair={canTarget}
        class:border-amber-400={canTarget}
        class:shadow-[0_0_20px_rgba(251,191,36,0.45)]={canTarget}
        class:ring-2={canTarget}
        class:ring-amber-400={canTarget}
        class:border-amber-500={!canTarget && opp.is_active}
        class:shadow-[0_0_15px_rgba(245,158,11,0.2)]={!canTarget && opp.is_active}
        onclick={canTarget ? () => onTargetPlayer(opp.id) : undefined}
        onkeydown={canTarget
          ? (e) => {
              if (e.key === 'Enter' || e.key === ' ') {
                e.preventDefault();
                onTargetPlayer(opp.id);
              }
            }
          : undefined}
      >

        <!-- Deck Color Accent -->
        <div class="h-1.5 w-full" style="background-color: {deckColor};"></div>

        <div class="px-4 py-3 flex flex-col gap-2">
          <div class="flex justify-between items-center gap-4">
            <div class="font-mono text-sm font-bold uppercase tracking-widest text-white truncate drop-shadow-md">
              {opp.name}
            </div>
            
            <div class="flex items-center gap-3 text-xs font-mono text-slate-300">
              <div class="flex items-center gap-1.5" title="Cards in Hand: {opp.hand_size || 0}">
                <span class="opacity-60 text-[10px]">HAND</span>
                <div class="flex -space-x-1.5 items-center">
                  {#if (opp.hand_size || 0) === 0}
                    <span class="font-bold text-slate-500">0</span>
                  {:else}
                    {#each Array(Math.min(7, opp.hand_size || 0)) as _}
                      <div class="w-2.5 h-3.5 bg-slate-300 rounded-[1px] shadow-sm border border-slate-500/50"></div>
                    {/each}
                    {#if (opp.hand_size || 0) > 7}
                      <span class="text-[10px] font-bold text-white ml-1.5">+{opp.hand_size - 7}</span>
                    {/if}
                  {/if}
                </div>
              </div>
              <button class="flex items-center gap-1 pointer-events-auto hover:text-amber-400 transition-colors cursor-pointer" 
                      title="Graveyard" 
                      onclick={() => onOpenDiscard(opp.id)}>
                <span class="opacity-60 text-[10px]">GRAVE</span>
                <span class="font-bold text-white">{opp.graveyard_size || 0}</span>
              </button>
            </div>
          </div>

          <!-- Health Bar -->
          <div class="flex items-center gap-2 mt-1">
            <div
              class="font-mono text-lg font-black text-red-400 leading-none life-number"
              class:life-pulse-damage={lifePulse[opp.id] === 'damage'}
              class:life-pulse-gain={lifePulse[opp.id] === 'gain'}
            >{opp.life}</div>
            <div class="flex-1 h-1.5 bg-slate-900 rounded-full overflow-hidden">
              <div class="h-full bg-red-500 transition-all duration-300" style="width: {lifePct}%"></div>
            </div>
          </div>

          {#if ruleset === 'syndicate'}
            <div class="flex items-center gap-3 mt-1 font-mono text-[11px] font-bold">
              <span class={cashOf(opp) < 0 ? 'text-red-400' : 'text-emerald-400'} title="Cash. Negative is debt.">
                {cashOf(opp) < 0 ? `-$${-cashOf(opp)}` : `$${cashOf(opp)}`}
              </span>
              <span class={heatTone(opp.heat ?? 0)} title="Heat. A raid comes at {HEAT_LINE}.">
                HEAT {opp.heat ?? 0}
              </span>
            </div>
          {:else if manaBlocks.length > 0}
            <div class="flex flex-wrap gap-0 mt-1 max-w-[180px]">
              {#each manaBlocks.slice(0, 14) as block (block.id)}
                <svg width="14" height="14" viewBox="0 0 24 24" class="overflow-visible -mr-1" style="filter: drop-shadow(0 0 3px {manaColors[block.color]}AA)">
                  <polygon points="12,2 20.7,7 20.7,17 12,22 3.3,17 3.3,7"
                           fill={block.active ? manaColors[block.color] : 'transparent'}
                           stroke={manaColors[block.color]}
                           stroke-width={block.active ? "0" : "2"}
                           stroke-dasharray={block.active ? "none" : "3, 2"} />
                </svg>
              {/each}
              {#if manaBlocks.length > 14}
                <div class="flex items-center text-[10px] font-mono text-slate-400 ml-2 font-bold">
                  +{manaBlocks.length - 14}
                </div>
              {/if}
            </div>
          {/if}

          <!-- Operator Damage Scoreboard (Operator format: 21 from a single op = lethal) -->
          {#if opDmgBadges.length > 0}
            <div class="flex flex-wrap gap-1 mt-1" title="Operator damage taken (21 = lethal)">
              {#each opDmgBadges as badge (badge.attackerId)}
                <span
                  class="px-1.5 py-0.5 rounded text-[10px] font-mono font-bold uppercase tracking-wide"
                  class:bg-red-600={badge.tier === 'high'}
                  class:text-white={badge.tier !== 'low'}
                  class:bg-amber-600={badge.tier === 'mid'}
                  class:bg-slate-700={badge.tier === 'low'}
                  class:text-slate-300={badge.tier === 'low'}
                >{badge.attackerName} {badge.dmg}/21</span>
              {/each}
            </div>
          {/if}

          <!-- Operator name + loyalty (with drain pulse animation) -->
          {#if oppOp && oppOp.loyalty != null}
            <div class="flex items-center gap-1.5 mt-1 text-[10px] font-mono">
              <span class="opacity-60 uppercase tracking-wide">OP</span>
              <span class="text-white font-bold truncate max-w-[140px]" title={oppOp.name}>{oppOp.name}</span>
              <span
                class="ml-auto px-1.5 py-0.5 rounded font-bold loyalty-counter"
                class:loyalty-pulse-drain={loyaltyPulse[opp.id] === 'drain'}
              >♦ {oppOp.loyalty}</span>
            </div>
          {/if}
        </div>
      </div>
    {/each}
  </div>

  <!-- Bottom Left: Player HUD -->
  {#if viewer}
    {@const deckColor = getPlayerColor(viewer)}
    {@const maxLife = getMaxLife(viewer)}
    {@const lifePct = Math.max(0, Math.min(100, (viewer.life / maxLife) * 100))}
    {@const manaBlocks = getManaBlocks(viewer)}
    {@const opDmgBadgesViewer = getOperatorDamageBadges(viewer)}
    {@const myOp = viewer.operator_zone?.[0]}
    {@const canTargetSelf = canTargetPlayer(viewer.id)}
    <div class="absolute bottom-8 left-8 flex flex-col items-start">
      <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
      <div
        role={canTargetSelf ? 'button' : undefined}
        tabindex={canTargetSelf ? 0 : undefined}
        class="flex flex-col bg-slate-950/90 backdrop-blur-md border border-white/10 shadow-2xl transition-all rounded-lg overflow-hidden min-w-[340px]"
        class:pointer-events-auto={canTargetSelf}
        class:cursor-crosshair={canTargetSelf}
        class:border-amber-400={canTargetSelf}
        class:shadow-[0_0_20px_rgba(251,191,36,0.45)]={canTargetSelf}
        class:ring-2={canTargetSelf}
        class:ring-amber-400={canTargetSelf}
        class:border-amber-500={!canTargetSelf && viewer.is_active}
        class:shadow-[0_0_20px_rgba(245,158,11,0.3)]={!canTargetSelf && viewer.is_active}
        onclick={canTargetSelf ? () => onTargetPlayer(viewer.id) : undefined}
        onkeydown={canTargetSelf
          ? (e) => {
              if (e.key === 'Enter' || e.key === ' ') {
                e.preventDefault();
                onTargetPlayer(viewer.id);
              }
            }
          : undefined}
      >
        
        <!-- Deck Color Accent -->
        <div class="h-2 w-full" style="background-color: {deckColor}; box-shadow: 0 0 10px {deckColor}80;"></div>

        <div class="px-6 py-4 flex flex-col gap-2">
          <div class="flex justify-between items-center gap-6">
            <div class="font-mono text-lg font-bold uppercase tracking-widest text-white truncate drop-shadow-md">
              {viewer.name}
            </div>
            
            <div class="flex items-center gap-5 text-sm font-mono text-slate-300">
              <div class="flex items-center gap-1.5" title="Cards in Hand: {viewer.hand_size || 0}">
                <span class="opacity-60 text-xs">HAND</span>
                <div class="flex -space-x-2 items-center">
                  {#if (viewer.hand_size || 0) === 0}
                    <span class="font-bold text-slate-500">0</span>
                  {:else}
                    {#each Array(Math.min(7, viewer.hand_size || 0)) as _}
                      <div class="w-3.5 h-5 bg-slate-300 rounded-[2px] shadow-sm border border-slate-500/50"></div>
                    {/each}
                    {#if (viewer.hand_size || 0) > 7}
                      <span class="text-xs font-bold text-white ml-2">+{viewer.hand_size - 7}</span>
                    {/if}
                  {/if}
                </div>
              </div>
              <button class="flex items-center gap-1 pointer-events-auto hover:text-amber-400 transition-colors cursor-pointer" 
                      title="Graveyard" 
                      onclick={() => onOpenDiscard(viewer.id)}>
                <span class="opacity-60 text-xs">GRAVE</span>
                <span class="font-bold text-white">{viewer.graveyard_size || 0}</span>
              </button>
            </div>
          </div>

          <!-- Health Bar -->
          <div class="flex items-center gap-3 mt-1">
            <div
              class="font-mono text-3xl font-black text-red-400 leading-none drop-shadow-[0_0_8px_rgba(248,113,113,0.4)] life-number"
              class:life-pulse-damage={lifePulse[viewer.id] === 'damage'}
              class:life-pulse-gain={lifePulse[viewer.id] === 'gain'}
            >
              {viewer.life}
            </div>
            <div class="flex-1 h-2 bg-slate-900 rounded-full overflow-hidden shadow-inner">
              <div class="h-full bg-gradient-to-r from-red-600 to-red-400 transition-all duration-300" style="width: {lifePct}%"></div>
            </div>
          </div>

          {#if ruleset === 'syndicate'}
            <div class="flex items-center gap-4 mt-2 font-mono text-sm font-bold">
              <span class={cashOf(viewer) < 0 ? 'text-red-400' : 'text-emerald-400'} title="Cash. Negative is debt.">
                {cashOf(viewer) < 0 ? `-$${-cashOf(viewer)}` : `$${cashOf(viewer)}`}
              </span>
              <span class={heatTone(viewer.heat ?? 0)} title="Heat. A raid comes at {HEAT_LINE}.">
                HEAT {viewer.heat ?? 0}
              </span>
            </div>
          {:else if manaBlocks.length > 0}
            <div class="flex flex-wrap gap-0 mt-2 max-w-[250px]">
              {#each manaBlocks.slice(0, 18) as block (block.id)}
                <svg width="20" height="20" viewBox="0 0 24 24" class="overflow-visible -mr-1.5" style="filter: drop-shadow(0 0 4px {manaColors[block.color]}AA)">
                  <polygon points="12,2 20.7,7 20.7,17 12,22 3.3,17 3.3,7"
                           fill={block.active ? manaColors[block.color] : 'transparent'}
                           stroke={manaColors[block.color]}
                           stroke-width={block.active ? "0" : "2"}
                           stroke-dasharray={block.active ? "none" : "3, 2"} />
                </svg>
              {/each}
              {#if manaBlocks.length > 18}
                <div class="flex items-center justify-center text-sm font-mono text-slate-400 ml-2 font-bold">
                  +{manaBlocks.length - 18}
                </div>
              {/if}
            </div>
          {/if}

          <!-- Operator Damage Scoreboard (Operator format: 21 from a single op = lethal) -->
          {#if opDmgBadgesViewer.length > 0}
            <div class="flex flex-wrap gap-1 mt-2" title="Operator damage taken (21 = lethal)">
              {#each opDmgBadgesViewer as badge (badge.attackerId)}
                <span
                  class="px-2 py-0.5 rounded text-xs font-mono font-bold uppercase tracking-wide"
                  class:bg-red-600={badge.tier === 'high'}
                  class:text-white={badge.tier !== 'low'}
                  class:bg-amber-600={badge.tier === 'mid'}
                  class:bg-slate-700={badge.tier === 'low'}
                  class:text-slate-300={badge.tier === 'low'}
                >{badge.attackerName} {badge.dmg}/21</span>
              {/each}
            </div>
          {/if}

          <!-- Operator name + loyalty (with drain pulse animation) -->
          {#if myOp && myOp.loyalty != null}
            <div class="flex items-center gap-2 mt-2 text-xs font-mono">
              <span class="opacity-60 uppercase tracking-wide">OPERATOR</span>
              <span class="text-white font-bold truncate max-w-[180px]" title={myOp.name}>{myOp.name}</span>
              <span
                class="ml-auto px-2 py-0.5 rounded font-bold loyalty-counter"
                class:loyalty-pulse-drain={loyaltyPulse[viewer.id] === 'drain'}
              >♦ {myOp.loyalty}</span>
            </div>
          {/if}
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  .life-number {
    transition: color 0.3s ease-out;
    transform-origin: center;
  }
  .life-pulse-damage {
    animation: life-pulse-damage 0.6s ease-out;
    color: #fca5a5;
  }
  .life-pulse-gain {
    animation: life-pulse-gain 0.6s ease-out;
    color: #86efac;
  }
  @keyframes life-pulse-damage {
    0% {
      transform: scale(1);
      text-shadow: 0 0 0 rgba(239, 68, 68, 0);
    }
    30% {
      transform: scale(1.4);
      text-shadow: 0 0 24px rgba(239, 68, 68, 0.95), 0 0 8px rgba(239, 68, 68, 0.9);
      color: #fee2e2;
    }
    100% {
      transform: scale(1);
      text-shadow: 0 0 0 rgba(239, 68, 68, 0);
    }
  }
  @keyframes life-pulse-gain {
    0% {
      transform: scale(1);
      text-shadow: 0 0 0 rgba(34, 197, 94, 0);
    }
    30% {
      transform: scale(1.3);
      text-shadow: 0 0 24px rgba(34, 197, 94, 0.9), 0 0 8px rgba(34, 197, 94, 0.8);
      color: #dcfce7;
    }
    100% {
      transform: scale(1);
      text-shadow: 0 0 0 rgba(34, 197, 94, 0);
    }
  }

  .loyalty-counter {
    background-color: rgba(99, 102, 241, 0.18);
    color: #c7d2fe;
    border: 1px solid rgba(99, 102, 241, 0.45);
    transition: color 0.3s ease-out, background-color 0.3s ease-out;
    transform-origin: center;
  }
  .loyalty-pulse-drain {
    animation: loyalty-pulse-drain 0.7s ease-out;
  }
  @keyframes loyalty-pulse-drain {
    0% {
      transform: scale(1);
      background-color: rgba(99, 102, 241, 0.18);
      color: #c7d2fe;
      box-shadow: 0 0 0 rgba(239, 68, 68, 0);
    }
    20% {
      transform: scale(1.4);
      background-color: rgba(239, 68, 68, 0.55);
      color: #fee2e2;
      box-shadow: 0 0 18px rgba(239, 68, 68, 0.85);
    }
    100% {
      transform: scale(1);
      background-color: rgba(99, 102, 241, 0.18);
      color: #c7d2fe;
      box-shadow: 0 0 0 rgba(239, 68, 68, 0);
    }
  }
</style>
