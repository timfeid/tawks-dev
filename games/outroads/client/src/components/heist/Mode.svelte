<script lang="ts">
  import { onDestroy } from 'svelte';
  import { Map as MapIcon, Volume2, VolumeX } from '@lucide/svelte';
  import TouchControls from '../TouchControls.svelte';
  import { HeistGame, type HeistHud } from '../../heist/heistGame';
  import type { DriveConnection } from '../../lib/net.svelte';
  import Hud from './Hud.svelte';
  import Map from './Map.svelte';
  import Results from './Results.svelte';
  import Setup from './Setup.svelte';
  import Take from './Take.svelte';

  let { conn, units, touch }: { conn: DriveConnection; units: 'mph' | 'kmh'; touch: boolean } = $props();

  let canvas = $state<HTMLCanvasElement>();
  let labels = $state<HTMLDivElement>();
  let game = $state<HeistGame | null>(null);
  let hud = $state<HeistHud | null>(null);
  let mapOpen = $state(false);
  let muted = $state(false);
  let notices = $state<{ id: number; text: string }[]>([]);
  let noticeId = 0;
  let showResults = $state(false);

  const view = $derived(conn.heist);
  const phase = $derived(view?.phase.phase);

  $effect(() => {
    if (!canvas || !labels || game) return;
    const g = new HeistGame(canvas, labels, conn);
    g.onHud = (h) => (hud = h);
    g.onPress = (p) => {
      if (p === 'map') mapOpen = !mapOpen;
    };
    g.onNotice = (text) => {
      const id = ++noticeId;
      notices = [...notices.slice(-3), { id, text }];
      setTimeout(() => (notices = notices.filter((n) => n.id !== id)), 6000);
    };
    game = g;
    (window as unknown as { __heist: unknown }).__heist = { game: g, conn };
  });

  $effect(() => {
    if (game && view) game.sync(view);
  });

  $effect(() => {
    if (game) {
      game.sound.muted = muted;
      game.sirens.muted = muted;
    }
  });

  const outcome = $derived(view?.phase.phase === 'ended' ? view.phase.results.outcome : null);

  $effect(() => {
    if (outcome) {
      const t = setTimeout(() => (showResults = true), outcome === 'delivered' ? 4500 : 1200);
      return () => clearTimeout(t);
    }
    showResults = false;
    if (game) game.cinematic = null;
  });

  onDestroy(() => game?.dispose());
</script>

<div class="mode">
  <canvas bind:this={canvas} class="view"></canvas>
  <div class="labels" bind:this={labels}></div>
  {#if view}
    {#if phase === 'setup' || phase === 'loading'}
      <Setup {conn} {view} />
    {:else if phase === 'running' && hud}
      <Hud {conn} {view} {hud} {units} />
      <Take {conn} {view} />
      <div class="corner">
        <button class="chip icon" onclick={() => (mapOpen = !mapOpen)} aria-label="Map (M)"><MapIcon size={18} /></button>
        <button class="chip icon" onclick={() => (muted = !muted)} aria-label={muted ? 'Sound on' : 'Sound off'}>{#if muted}<VolumeX size={18} />{:else}<Volume2 size={18} />{/if}</button>
      </div>
      {#if view.me?.arrested}<div class="center panel message">You were arrested. Watch the rest of the run.</div>{/if}
      {#if mapOpen && game}<Map {conn} {view} {game} onclose={() => (mapOpen = false)} />{/if}
      {#if touch && game}<TouchControls input={game.input} onhonk={() => game?.input.presses.push('honk')} />{/if}
      {#if touch}<button class="chip use" onclick={() => game?.input.presses.push('interact')}>Use</button>{/if}
    {:else if phase === 'ended' && showResults && view.phase.phase === 'ended'}
      <Results {conn} {view} results={view.phase.results} />
    {/if}
  {/if}
  <div class="notices">
    {#each notices as n (n.id)}<div class="panel notice">{n.text}</div>{/each}
  </div>
  <footer class="credits">© OpenStreetMap contributors, OpenFreeMap, © OpenMapTiles, terrain: Mapzen, USGS 3DEP, SRTM, GMTED2010 and others</footer>
</div>

<style>
  .mode {
    position: absolute;
    inset: 0;
  }
  .view {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    display: block;
    touch-action: none;
  }
  .labels {
    position: absolute;
    inset: 0;
    pointer-events: none;
    overflow: hidden;
  }
  .corner {
    position: absolute;
    right: 16px;
    top: calc(14px + env(safe-area-inset-top));
    display: flex;
    gap: 6px;
    z-index: 6;
  }
  .use {
    position: absolute;
    right: 16px;
    bottom: calc(150px + env(safe-area-inset-bottom));
    z-index: 7;
  }
  .center {
    position: absolute;
    left: 50%;
    top: 40%;
    transform: translate(-50%, -50%);
    padding: 14px 18px;
    z-index: 6;
  }
  .notices {
    position: absolute;
    top: calc(64px + env(safe-area-inset-top));
    right: 16px;
    display: flex;
    flex-direction: column;
    gap: 6px;
    width: min(320px, calc(100% - 32px));
    z-index: 12;
    pointer-events: none;
  }
  .notice {
    padding: 8px 12px;
    font-size: 14px;
  }
  .credits {
    position: absolute;
    right: 8px;
    bottom: 4px;
    font-size: 10.5px;
    color: rgba(238, 242, 246, 0.6);
    pointer-events: none;
    z-index: 3;
  }
</style>
