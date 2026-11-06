<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import type { Game } from '../lib/game';

  let { game, size, heading }: { game: Game; size: number; heading: number } = $props();

  let canvas = $state<HTMLCanvasElement>();
  let range = $state(260);
  let timer: ReturnType<typeof setInterval> | null = null;

  onMount(() => {
    timer = setInterval(() => {
      if (!canvas) return;
      const dpr = Math.min(2, window.devicePixelRatio || 1);
      if (canvas.width !== Math.round(size * dpr)) {
        canvas.width = Math.round(size * dpr);
        canvas.height = Math.round(size * dpr);
      }
      const ctx = canvas.getContext('2d');
      if (ctx) game.drawMinimap(ctx, size, range);
    }, 120);
  });
  onDestroy(() => timer && clearInterval(timer));

  function zoom() {
    range = range === 260 ? 520 : range === 520 ? 130 : 260;
  }
</script>

<button class="minimap" style:width="{size}px" style:height="{size}px" onclick={zoom} aria-label="Minimap, tap to zoom">
  <canvas bind:this={canvas} style:width="{size}px" style:height="{size}px"></canvas>
  <span class="north" style:transform="rotate({-heading}rad)"><span>N</span></span>
</button>

<style>
  .minimap {
    position: relative;
    padding: 0;
    border: 2px solid rgba(255, 255, 255, 0.18);
    border-radius: 50%;
    background: none;
    box-shadow: 0 10px 30px rgba(0, 0, 0, 0.35);
  }
  canvas {
    display: block;
    border-radius: 50%;
  }
  .north {
    position: absolute;
    inset: -2px;
    pointer-events: none;
  }
  .north span {
    position: absolute;
    left: 50%;
    top: -9px;
    transform: translateX(-50%);
    width: 20px;
    height: 20px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    background: var(--stop);
    color: #fff;
    font-family: var(--font-sign);
    font-size: 12px;
    font-weight: 700;
  }
</style>
