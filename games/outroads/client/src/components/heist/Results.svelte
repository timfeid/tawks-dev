<script lang="ts">
  import type { DriveConnection } from '../../lib/net.svelte';
  import type { HeistView, Results } from '../../heist/protocol';

  let { conn, view, results }: { conn: DriveConnection; view: HeistView; results: Results } = $props();

  const names = (ids: string[]) => ids.map((id) => view.players.find((p) => p.id === id)?.name ?? id).join(', ') || 'nobody';
  const title = $derived(results.outcome === 'delivered' ? 'Delivered' : results.outcome === 'arrested' ? 'Arrested' : 'Out of time');
  const who = $derived(results.outcome === 'delivered' ? `The crew wins: ${names(results.thieves)}` : `The police win: ${names(results.cops)}`);
  const mins = $derived(`${Math.floor(results.seconds / 60)}:${String(Math.floor(results.seconds % 60)).padStart(2, '0')}`);
</script>

<div class="backdrop">
  <section class="panel results">
    <h1 class:won={results.outcome === 'delivered'}>{title}</h1>
    <p>{who}</p>
    {#if results.car}<p>The car: a {results.car}</p>{/if}
    <dl>
      <dt>Payout</dt><dd>${results.payout.toLocaleString()} in-game cash</dd>
      <dt>Condition</dt><dd>{Math.round(results.condition * 100)}%</dd>
      <dt>Time</dt><dd>{mins}</dd>
      <dt>Driven</dt><dd>{(results.stats.driven / 1000).toFixed(1)} km</dd>
      <dt>Peak heat</dt><dd>{Math.round(results.stats.peak_heat)}</dd>
      <dt>Pursuits</dt><dd>{results.stats.pursuits}</dd>
      <dt>Spikes hit</dt><dd>{results.stats.spikes_hit}</dd>
      <dt>Spin outs</dt><dd>{results.stats.spin_outs}</dd>
      <dt>Witnesses</dt><dd>{results.stats.witnesses}</dd>
      <dt>Plate reads</dt><dd>{results.stats.plate_hits}</dd>
      <dt>Caught on foot</dt><dd>{results.stats.foot_arrests}</dd>
    </dl>
    {#if conn.host}
      <div class="row">
        <button class="primary" onclick={() => conn.heistSend({ op: 'rematch' })}>Another run</button>
        <button class="ghost" onclick={() => conn.heistOpen(false)}>Back to free roam</button>
      </div>
    {:else}
      <p class="note">The host picks what is next.</p>
    {/if}
  </section>
</div>

<style>
  .backdrop {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    padding: 16px;
    background: rgba(5, 8, 14, 0.55);
    z-index: 20;
  }
  .results {
    width: min(460px, 100%);
    padding: 22px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  h1 {
    margin: 0;
    font-family: var(--font-sign);
    font-size: 36px;
    text-transform: uppercase;
    color: #ff7a5a;
  }
  h1.won {
    color: var(--go);
  }
  p {
    margin: 0;
    color: var(--text-dim);
  }
  dl {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 4px 14px;
    margin: 8px 0;
    font-size: 14px;
  }
  dt {
    color: var(--text-faint);
  }
  dd {
    margin: 0;
  }
  .row {
    display: flex;
    gap: 8px;
  }
  .note {
    font-size: 13px;
  }
</style>
