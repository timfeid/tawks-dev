<script lang="ts">
  import { Eye, EyeOff, Lock, Minus, Plus } from '@lucide/svelte';
  import Die from './Die.svelte';
  import RollLog from './RollLog.svelte';
  import { QUICK_DICE, quickFormula, signed } from '../lib/format';
  import type { TableConnection } from '../lib/table.svelte';
  import type { RollView, Visibility } from '../lib/types';

  let { conn }: { conn: TableConnection } = $props();

  let mod = $state(0);
  let edge = $state<'advantage' | 'disadvantage' | null>(null);
  let visibility = $state<Visibility>('public');
  let formula = $state('');

  const isGm = $derived(conn.isGm);
  const choices = $derived<{ value: Visibility; label: string; icon: typeof Eye }[]>(
    isGm
      ? [
          { value: 'public', label: 'Table', icon: Eye },
          { value: 'gm', label: 'Hidden', icon: EyeOff },
        ]
      : [
          { value: 'public', label: 'Table', icon: Eye },
          { value: 'gm', label: 'To GM', icon: Lock },
          { value: 'me', label: 'Just me', icon: EyeOff },
        ],
  );

  $effect(() => {
    if (!choices.some((c) => c.value === visibility)) visibility = 'public';
  });

  type Open = Extract<RollView, { shown: 'open' }>;
  const latest = $derived.by(() => {
    const rolls = conn.table?.rolls ?? [];
    for (let i = rolls.length - 1; i >= 0; i--) {
      const roll = rolls[i];
      if (roll.shown === 'open') return roll as Open;
    }
    return null;
  });
  const tumbling = $derived(!!latest && conn.fresh.includes(latest.id) && Date.now() - conn.freshAt < 1500);

  function roll(text: string) {
    const trimmed = text.trim();
    if (!trimmed) return;
    conn.act({ type: 'roll', formula: trimmed, visibility });
  }

  function rollTyped() {
    roll(formula);
  }

  function toggle(next: 'advantage' | 'disadvantage') {
    edge = edge === next ? null : next;
  }

  function dieTone(sides: number, value: number, kept: boolean) {
    if (sides !== 20 || !kept) return 'bone';
    if (value === 20) return 'brass';
    if (value === 1) return 'ember';
    return 'bone';
  }
</script>

<section class="tray" aria-label="Dice tray">
  <div class="felt">
    {#if latest}
      {#key latest.id}
        <div class="landing" class:secret={latest.visibility !== 'public'}>
          <div class="dice">
            {#each latest.roll.parts as part, i (i)}
              {#if part.kind === 'dice'}
                {#each part.dice.slice(0, 12) as die, j (j)}
                  <Die sides={part.sides} value={die.value} kept={die.kept} size={52} tumble={tumbling} tone={dieTone(part.sides, die.value, die.kept)} />
                {/each}
                {#if part.dice.length > 12}<span class="more">+{part.dice.length - 12}</span>{/if}
              {:else}
                <span class="flat number">{part.negative ? '-' : '+'}{part.value}</span>
              {/if}
            {/each}
          </div>
          <div class="result">
            <span class="total number" class:crit={latest.roll.natural === 'twenty'} class:fumble={latest.roll.natural === 'one'}>{latest.roll.total}</span>
            <span class="who">
              {latest.by_name}
              <span class="formula number">{latest.roll.formula}</span>
              {#if latest.roll.label}<span class="tag">{latest.roll.label}</span>{/if}
              {#if latest.roll.natural === 'twenty'}<span class="mark crit">Natural 20</span>{/if}
              {#if latest.roll.natural === 'one'}<span class="mark fumble">Natural 1</span>{/if}
              {#if latest.visibility === 'gm'}<span class="mark hidden">Hidden</span>{/if}
              {#if latest.visibility === 'me'}<span class="mark hidden">Just you</span>{/if}
            </span>
          </div>
        </div>
      {/key}
    {:else}
      <p class="empty">Dice land here. Pick a die or type a roll.</p>
    {/if}
  </div>

  <div class="quick" role="group" aria-label="Quick dice">
    {#each QUICK_DICE as sides (sides)}
      <button class="die-button" onclick={() => roll(quickFormula(sides, mod, edge))} title={`Roll ${quickFormula(sides, mod, edge)}`}>
        <Die {sides} size={38} tone="plain" />
      </button>
    {/each}
  </div>

  <div class="controls">
    <div class="segment" role="group" aria-label="Advantage">
      <button class:on={edge === 'advantage'} aria-pressed={edge === 'advantage'} onclick={() => toggle('advantage')}>Adv</button>
      <button class:on={edge === 'disadvantage'} aria-pressed={edge === 'disadvantage'} onclick={() => toggle('disadvantage')}>Dis</button>
    </div>
    <div class="stepper" aria-label="Modifier">
      <button class="btn quiet icon" aria-label="Lower the modifier" onclick={() => (mod = Math.max(-20, mod - 1))}><Minus size={16} /></button>
      <span class="number mod">{signed(mod)}</span>
      <button class="btn quiet icon" aria-label="Raise the modifier" onclick={() => (mod = Math.min(20, mod + 1))}><Plus size={16} /></button>
    </div>
    <div class="segment" role="group" aria-label="Who sees it">
      {#each choices as choice (choice.value)}
        {@const Icon = choice.icon}
        <button class:on={visibility === choice.value} aria-pressed={visibility === choice.value} onclick={() => (visibility = choice.value)}>
          <Icon size={14} />{choice.label}
        </button>
      {/each}
    </div>
  </div>

  <div class="typed">
    <input
      class="field"
      bind:value={formula}
      placeholder="2d6+3, 4d6dl1, 1d20+5 adv stealth"
      aria-label="Roll formula"
      onkeydown={(e) => e.key === 'Enter' && rollTyped()}
    />
    <button class="btn primary" onclick={rollTyped} disabled={!formula.trim()}>Roll</button>
  </div>

  <RollLog {conn} />
</section>

<style>
  .tray {
    display: flex;
    flex-direction: column;
    gap: 12px;
    min-height: 0;
    height: 100%;
  }

  .felt {
    position: relative;
    min-height: 132px;
    padding: 14px;
    border-radius: 18px;
    border: 3px solid var(--wood-600);
    background:
      radial-gradient(ellipse at 50% 30%, rgba(255, 255, 255, 0.05), transparent 70%),
      repeating-radial-gradient(circle at 30% 40%, rgba(255, 255, 255, 0.012) 0 1px, transparent 1px 3px),
      var(--felt);
    box-shadow:
      inset 0 0 0 1px var(--felt-line),
      inset 0 10px 24px rgba(0, 0, 0, 0.55),
      0 1px 0 rgba(255, 220, 170, 0.08);
    display: flex;
    align-items: center;
  }

  .landing {
    display: flex;
    flex-direction: column;
    gap: 10px;
    width: 100%;
  }

  .landing.secret {
    background: repeating-linear-gradient(135deg, rgba(169, 203, 224, 0.05) 0 8px, transparent 8px 16px);
    border-radius: 10px;
  }

  .dice {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
  }

  .flat {
    font-size: 22px;
    font-weight: 700;
    color: var(--parchment);
    padding: 0 4px;
  }

  .more {
    color: var(--text-dim);
    font-weight: 700;
  }

  .result {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .total {
    font-family: var(--font-display);
    font-size: 44px;
    line-height: 1;
    font-weight: 700;
    color: var(--parchment);
    text-shadow: 0 2px 0 rgba(0, 0, 0, 0.5);
  }

  .total.crit {
    color: var(--brass-bright);
  }

  .total.fumble {
    color: var(--ember);
  }

  .who {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
    font-weight: 700;
    color: var(--text);
  }

  .formula {
    font-size: 13px;
    color: var(--text-dim);
    font-weight: 500;
  }

  .tag {
    font-size: 13px;
    color: var(--text-dim);
    font-style: italic;
    font-weight: 500;
  }

  .mark {
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    padding: 2px 7px;
    border-radius: 999px;
  }

  .mark.crit {
    color: var(--wood-950);
    background: var(--brass-bright);
  }

  .mark.fumble {
    color: #2a0d04;
    background: var(--ember);
  }

  .mark.hidden {
    color: var(--wood-950);
    background: var(--frost);
  }

  .empty {
    margin: 0 auto;
    color: rgba(244, 234, 217, 0.55);
    font-style: italic;
  }

  .quick {
    display: grid;
    grid-template-columns: repeat(7, minmax(0, 1fr));
    gap: 6px;
  }

  .die-button {
    display: grid;
    place-items: center;
    height: 52px;
    border-radius: var(--radius-small);
    border: 1px solid rgba(214, 165, 79, 0.25);
    background: linear-gradient(180deg, var(--wood-700), var(--wood-800));
    transition: transform 120ms, border-color 120ms;
  }

  .die-button:hover {
    border-color: var(--brass);
    transform: translateY(-1px);
  }

  .die-button:active {
    transform: translateY(1px) rotate(-6deg);
  }

  .controls {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    align-items: center;
  }

  .segment {
    display: inline-flex;
    border: 1px solid rgba(214, 165, 79, 0.3);
    border-radius: var(--radius-small);
    overflow: hidden;
  }

  .segment button {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    height: calc(var(--control) - 2px);
    padding: 0 11px;
    border: none;
    background: rgba(10, 6, 3, 0.35);
    color: var(--text-dim);
    font-weight: 700;
    font-size: 13px;
  }

  .segment button + button {
    border-left: 1px solid rgba(214, 165, 79, 0.2);
  }

  .segment button.on {
    color: var(--wood-950);
    background: var(--brass);
  }

  .stepper {
    display: inline-flex;
    align-items: center;
    border: 1px solid rgba(214, 165, 79, 0.3);
    border-radius: var(--radius-small);
    height: var(--control);
  }

  .mod {
    min-width: 34px;
    text-align: center;
    font-weight: 700;
  }

  .typed {
    display: flex;
    gap: 8px;
  }

  .typed .field {
    flex: 1;
  }
</style>
