<script lang="ts">
  import { Flame, Siren, Timer } from '@lucide/svelte';
  import type { HeistHud } from '../../heist/heistGame';
  import type { DriveConnection } from '../../lib/net.svelte';
  import type { HeistView } from '../../heist/protocol';

  let { conn, view, hud, units }: { conn: DriveConnection; view: HeistView; hud: HeistHud; units: 'mph' | 'kmh' } = $props();

  const role = $derived(view.me?.role);
  const left = $derived(view.phase.phase === 'running' ? view.phase.left : 0);
  const clock = $derived(`${Math.floor(left / 60)}:${String(Math.floor(left % 60)).padStart(2, '0')}`);
  const speed = $derived(Math.round(Math.abs(hud.speed) * (units === 'mph' ? 2.23694 : 3.6)));
  const heat = $derived(Math.round(view.heat ?? 0));
  const mine = $derived(view.vehicles.find((v) => v.driver === conn.me || v.riders.includes(conn.me ?? '')));
  const RADIO = ['Need backup', 'In pursuit', 'Lost visual', 'Suspect on foot', 'Spikes set'];
</script>

<div class="top">
  <div class="chip clock"><Timer size={16} /> {clock}</div>
  {#if role === 'thief' && view.contract}
    <div class="chip contract" title="The buyer's order">{view.contract.text}</div>
    <div class="chip heat" class:hot={heat >= 45}><Flame size={16} /> Heat {heat}</div>
    {#if view.reported}<div class="chip warn">Reported stolen</div>{/if}
  {:else if role === 'cop'}
    <div class="chip wanted">{view.wanted ? `Wanted: ${view.wanted}` : 'No stolen car reported yet'}</div>
  {/if}
  {#if view.pursuit === 'pursuit'}<div class="chip chase"><Siren size={16} /> Pursuit</div>
  {:else if view.pursuit === 'search'}<div class="chip search">Police searching</div>{/if}
</div>

<div class="bottom">
  {#if hud.prompt}<div class="chip prompt"><kbd>E</kbd> {hud.prompt}</div>{/if}
  {#if hud.inspect}<div class="chip inspect" class:match={hud.inspect.matches}>{hud.inspect.text}</div>{/if}
  {#if hud.mode === 'drive' || hud.mode === 'ride'}
    <div class="speed panel"><span>{speed}</span> {units === 'mph' ? 'mph' : 'km/h'}</div>
    {#if hud.street}<div class="sign">{hud.street}</div>{/if}
  {/if}
  {#if mine}
    <div class="chip damage" title="The car's condition">
      Car {Math.round((mine.damage.panels.reduce((a, b) => a + b, 0) / 4) * 100)}%{mine.damage.tyres.includes('rim') ? ', on the rims' : mine.damage.tyres.includes('flat') ? ', flat' : ''}{mine.handling.smoke ? ', engine smoking' : ''}
    </div>
  {/if}
</div>

{#if role === 'cop'}
  <div class="cop panel">
    <button class="ghost small" onclick={() => conn.heistSend({ op: 'siren', on: !mine?.siren })}><kbd>G</kbd> Siren</button>
    <button class="ghost small" onclick={() => conn.heistSend({ op: 'spotlight', on: !mine?.spotlight })}><kbd>F</kbd> {hud.mode === 'foot' ? 'Flashlight' : 'Spotlight'}</button>
    <button class="ghost small" onclick={() => conn.heistSend({ op: 'spike' })}><kbd>B</kbd> Spikes</button>
    <div class="radio">
      {#each RADIO as text}<button class="ghost tiny" onclick={() => conn.heistSend({ op: 'ping', kind: 'radio', x: hud.x, z: hud.z, text })}>{text}</button>{/each}
    </div>
  </div>
{:else if role === 'thief'}
  <div class="crew panel">
    <button class="ghost tiny" onclick={() => conn.heistSend({ op: 'ping', kind: 'pickup', x: hud.x, z: hud.z, text: 'Pick me up here' })}>Call the car here</button>
    <button class="ghost tiny" onclick={() => conn.heistSend({ op: 'ping', kind: 'danger', x: hud.x, z: hud.z, text: 'Cops here' })}>Warn the crew</button>
  </div>
{/if}

<style>
  .top {
    position: absolute;
    top: calc(14px + env(safe-area-inset-top));
    left: 50%;
    transform: translateX(-50%);
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
    justify-content: center;
    max-width: calc(100% - 360px);
    z-index: 5;
    pointer-events: none;
  }
  .contract {
    color: var(--signal);
  }
  .heat.hot,
  .warn {
    color: #ff7a5a;
    border-color: #ff7a5a;
  }
  .chase {
    color: #fff;
    background: rgba(200, 30, 30, 0.75);
    border-color: #ff5b4a;
    animation: pulse 0.6s infinite alternate;
  }
  @keyframes pulse {
    to {
      background: rgba(30, 60, 200, 0.75);
    }
  }
  .search {
    color: #ffd24a;
  }
  .wanted {
    color: #9bbcff;
  }
  .bottom {
    position: absolute;
    left: 50%;
    bottom: calc(18px + env(safe-area-inset-bottom));
    transform: translateX(-50%);
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
    justify-content: center;
    max-width: calc(100% - 32px);
    z-index: 5;
  }
  kbd {
    font-family: ui-monospace, monospace;
    font-size: 12px;
    padding: 1px 6px;
    border: 1px solid var(--line-strong);
    border-radius: 4px;
  }
  .inspect.match {
    color: var(--go);
    border-color: var(--go);
  }
  .speed {
    padding: 4px 12px;
    font-size: 13px;
    color: var(--text-dim);
  }
  .speed span {
    font-family: var(--font-sign);
    font-size: 26px;
    color: var(--text);
  }
  .sign {
    padding: 6px 14px;
    border-radius: 8px;
    background: var(--sign);
    border: 2px solid rgba(255, 255, 255, 0.85);
    font-family: var(--font-sign);
    font-size: 19px;
    max-width: min(380px, 50vw);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .cop,
  .crew {
    position: absolute;
    left: 16px;
    bottom: calc(70px + env(safe-area-inset-bottom));
    padding: 10px;
    display: flex;
    flex-direction: column;
    gap: 6px;
    width: 200px;
    z-index: 5;
  }
  .small {
    height: 32px;
    font-size: 13px;
    display: flex;
    gap: 6px;
    align-items: center;
  }
  .tiny {
    height: 28px;
    font-size: 12px;
    padding: 0 8px;
  }
  .radio {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }
  @media (max-width: 640px) {
    .top {
      max-width: calc(100% - 140px);
      left: 12px;
      transform: none;
      justify-content: flex-start;
      top: calc(64px + env(safe-area-inset-top));
    }
    .cop,
    .crew {
      bottom: auto;
      top: calc(150px + env(safe-area-inset-top));
      width: 160px;
    }
  }
</style>
