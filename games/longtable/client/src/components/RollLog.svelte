<script lang="ts">
  import { EyeOff } from '@lucide/svelte';
  import { partsText, secretText } from '../lib/format';
  import type { TableConnection } from '../lib/table.svelte';

  let { conn }: { conn: TableConnection } = $props();

  const rolls = $derived([...(conn.table?.rolls ?? [])].reverse());

  function time(ms: number) {
    return new Date(ms).toLocaleTimeString([], { hour: 'numeric', minute: '2-digit' });
  }
</script>

<div class="log" aria-label="Roll log">
  <div class="head"><span class="label">Roll log</span></div>
  {#if rolls.length === 0}
    <p class="none">No rolls yet tonight.</p>
  {/if}
  <ol>
    {#each rolls as roll (roll.id)}
      {#if roll.shown === 'open'}
        <li class:mine={roll.by === conn.me} class:private={roll.visibility !== 'public'}>
          <span class="total number" class:crit={roll.roll.natural === 'twenty'} class:fumble={roll.roll.natural === 'one'}>{roll.roll.total}</span>
          <span class="body">
            <span class="line">
              <strong>{roll.by_name}</strong>
              <span class="number formula">{roll.roll.formula}</span>
              {#if roll.roll.label}<em>{roll.roll.label}</em>{/if}
              {#if roll.visibility === 'gm'}<span class="badge">{conn.table?.gm === roll.by ? 'Hidden' : 'To GM'}</span>{/if}
              {#if roll.visibility === 'me'}<span class="badge">Just you</span>{/if}
            </span>
            <span class="parts number">
              {#each roll.roll.parts as part, i (i)}
                {#if part.kind === 'dice'}
                  {part.negative ? ' - ' : i > 0 ? ' + ' : ''}(
                  {#each part.dice as die, j (j)}<span class:dropped={!die.kept}>{die.value}</span>{j < part.dice.length - 1 ? ', ' : ''}{/each}
                  )
                {:else}
                  {part.negative ? ' - ' : ' + '}{part.value}
                {/if}
              {/each}
            </span>
          </span>
          <time class="number">{time(roll.at_ms)}</time>
        </li>
      {:else}
        <li class="secret">
          <span class="total"><EyeOff size={16} /></span>
          <span class="body"><span class="line">{secretText(roll)}</span></span>
          <time class="number">{time(roll.at_ms)}</time>
        </li>
      {/if}
    {/each}
  </ol>
  <span class="sr-only" aria-live="polite">{rolls[0] && rolls[0].shown === 'open' ? `${rolls[0].by_name} rolled ${rolls[0].roll.total}, ${partsText(rolls[0].roll.parts)}` : ''}</span>
</div>

<style>
  .log {
    display: flex;
    flex-direction: column;
    min-height: 0;
    flex: 1;
  }

  .head {
    padding: 2px 2px 6px;
    border-bottom: 1px solid rgba(214, 165, 79, 0.2);
  }

  .none {
    color: var(--text-faint);
    font-style: italic;
    margin: 10px 2px;
  }

  ol {
    list-style: none;
    margin: 0;
    padding: 0;
    overflow-y: auto;
    min-height: 0;
    flex: 1;
  }

  li {
    display: grid;
    grid-template-columns: 46px 1fr auto;
    align-items: center;
    gap: 10px;
    padding: 8px 2px;
    border-bottom: 1px dashed rgba(214, 165, 79, 0.14);
  }

  li.mine .total {
    border-color: var(--brass);
  }

  .total {
    display: grid;
    place-items: center;
    height: 40px;
    border-radius: 10px;
    border: 1px solid rgba(214, 165, 79, 0.25);
    background: rgba(10, 6, 3, 0.35);
    font-size: 18px;
    font-weight: 700;
  }

  .total.crit {
    color: var(--wood-950);
    background: var(--brass-bright);
  }

  .total.fumble {
    color: #2a0d04;
    background: var(--ember);
  }

  .body {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .line {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    align-items: baseline;
  }

  .formula {
    color: var(--text-dim);
    font-size: 13px;
  }

  em {
    color: var(--text-dim);
    font-size: 13px;
  }

  .parts {
    color: var(--text-faint);
    font-size: 12px;
  }

  .dropped {
    text-decoration: line-through;
    opacity: 0.6;
  }

  .badge {
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--wood-950);
    background: var(--frost);
    padding: 1px 6px;
    border-radius: 999px;
  }

  li.secret {
    color: var(--frost);
    font-style: italic;
  }

  li.secret .total {
    color: var(--frost);
    border-style: dashed;
  }

  time {
    color: var(--text-faint);
    font-size: 11px;
  }
</style>
