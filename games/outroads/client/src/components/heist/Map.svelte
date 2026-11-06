<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { X } from '@lucide/svelte';
  import type { HeistGame } from '../../heist/heistGame';
  import type { DriveConnection } from '../../lib/net.svelte';
  import type { HeistView, PingKind } from '../../heist/protocol';

  let { conn, view, game, onclose }: { conn: DriveConnection; view: HeistView; game: HeistGame; onclose: () => void } = $props();

  let canvas = $state<HTMLCanvasElement>();
  let range = $state(1400);
  let kind = $state<PingKind>('mark');
  let timer: ReturnType<typeof setInterval> | null = null;
  const role = $derived(view.me?.role);
  const COLORS: Record<string, string> = { mark: '#ffffff', pickup: '#3fd08f', danger: '#ff5b4a', radio: '#5aa0ff', witness: '#ffd24a', reader: '#ff7a2a', report: '#ff3b3b', flagged: '#ffd24a', seen: '#ff3b3b', alarm: '#ffb547' };

  function centre() {
    return { x: game.hud.x, z: game.hud.z };
  }

  function draw() {
    const c = canvas;
    if (!c) return;
    const rect = c.getBoundingClientRect();
    const dpr = Math.min(2, devicePixelRatio || 1);
    if (c.width !== Math.round(rect.width * dpr)) {
      c.width = Math.round(rect.width * dpr);
      c.height = Math.round(rect.height * dpr);
    }
    const ctx = c.getContext('2d')!;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    const w = rect.width;
    const h = rect.height;
    ctx.fillStyle = '#0b0f15';
    ctx.fillRect(0, 0, w, h);
    const { x: cx, z: cz } = centre();
    const scale = Math.min(w, h) / 2 / range;
    const sx = (x: number) => w / 2 + (x - cx) * scale;
    const sz = (z: number) => h / 2 + (z - cz) * scale;
    ctx.strokeStyle = 'rgba(200,210,225,0.55)';
    ctx.lineCap = 'round';
    game.world.index.forEachRoad(cx, cz, range * 1.5, (s, o, half) => {
      ctx.lineWidth = Math.max(1, half * 2 * scale);
      ctx.beginPath();
      ctx.moveTo(sx(s[o]), sz(s[o + 1]));
      ctx.lineTo(sx(s[o + 2]), sz(s[o + 3]));
      ctx.stroke();
    });
    if (role === 'thief') {
      for (const t of view.contract?.towns ?? []) {
        ctx.strokeStyle = 'rgba(255,181,71,0.8)';
        ctx.setLineDash([6, 6]);
        ctx.lineWidth = 2;
        ctx.beginPath();
        ctx.arc(sx(t.x), sz(t.z), t.r * scale, 0, Math.PI * 2);
        ctx.stroke();
        ctx.setLineDash([]);
        ctx.fillStyle = '#ffb547';
        ctx.font = '600 12px Barlow, sans-serif';
        ctx.fillText(t.name, sx(t.x) + 4, sz(t.z));
      }
    }
    ctx.fillStyle = '#ff9d2e';
    ctx.beginPath();
    ctx.arc(sx(view.gate.x), sz(view.gate.z), 6, 0, Math.PI * 2);
    ctx.fill();
    ctx.fillText('Port gate', sx(view.gate.x) + 8, sz(view.gate.z) + 4);
    for (const r of conn.heistWorld?.readers ?? []) {
      ctx.fillStyle = '#ff4040';
      ctx.fillRect(sx(r.x) - 3, sz(r.z) - 3, 6, 6);
    }
    for (const p of view.pings) {
      ctx.fillStyle = COLORS[p.kind] ?? '#fff';
      ctx.globalAlpha = Math.max(0.3, 1 - p.age / 75);
      if (role === 'cop' && ['witness', 'report', 'reader', 'flagged', 'seen', 'alarm'].includes(p.kind)) {
        ctx.beginPath();
        ctx.arc(sx(p.x), sz(p.z), Math.max(10, 180 * scale), 0, Math.PI * 2);
        ctx.fillStyle = 'rgba(255,80,60,0.18)';
        ctx.fill();
        ctx.fillStyle = COLORS[p.kind];
      }
      ctx.beginPath();
      ctx.arc(sx(p.x), sz(p.z), 5, 0, Math.PI * 2);
      ctx.fill();
      ctx.fillText(p.text, sx(p.x) + 7, sz(p.z) - 6);
      ctx.globalAlpha = 1;
    }
    const data = game.mapData();
    for (const [, d] of data.dyn) {
      ctx.fillStyle = d.kind === 'cruiser' ? '#5aa0ff' : '#ffb547';
      ctx.beginPath();
      ctx.arc(sx(d.x), sz(d.z), 4, 0, Math.PI * 2);
      ctx.fill();
    }
    for (const d of data.ai.values()) {
      ctx.fillStyle = '#5aa0ff';
      ctx.beginPath();
      ctx.arc(sx(d.x), sz(d.z), 3, 0, Math.PI * 2);
      ctx.fill();
    }
    ctx.fillStyle = '#ffffff';
    ctx.beginPath();
    ctx.moveTo(w / 2 + Math.sin(game.hud.h) * 9, h / 2 - Math.cos(game.hud.h) * 9);
    ctx.lineTo(w / 2 + Math.sin(game.hud.h + 2.4) * 7, h / 2 - Math.cos(game.hud.h + 2.4) * 7);
    ctx.lineTo(w / 2 + Math.sin(game.hud.h - 2.4) * 7, h / 2 - Math.cos(game.hud.h - 2.4) * 7);
    ctx.fill();
  }

  function click(e: MouseEvent) {
    const c = canvas!;
    const rect = c.getBoundingClientRect();
    const scale = Math.min(rect.width, rect.height) / 2 / range;
    const { x: cx, z: cz } = centre();
    const x = cx + (e.clientX - rect.left - rect.width / 2) / scale;
    const z = cz + (e.clientY - rect.top - rect.height / 2) / scale;
    if (role === 'cop') conn.heistSend({ op: 'ping', kind: 'radio', x, z, text: 'Check here' });
    else conn.heistSend({ op: 'ping', kind, x, z, text: kind === 'pickup' ? 'Pick up here' : kind === 'danger' ? 'Cops here' : 'Here' });
  }

  onMount(() => {
    timer = setInterval(draw, 150);
  });
  onDestroy(() => timer && clearInterval(timer));
</script>

<div class="backdrop">
  <section class="panel map">
    <header>
      <strong>{role === 'cop' ? 'Dispatch' : 'Crew map'}</strong>
      {#if role === 'thief'}
        <div class="kinds">
          {#each [['mark', 'Mark'], ['pickup', 'Pick up'], ['danger', 'Danger']] as [k, label]}
            <button class="ghost tiny" class:on={kind === k} onclick={() => (kind = k as PingKind)}>{label}</button>
          {/each}
        </div>
      {/if}
      <div class="kinds">
        <button class="ghost tiny" onclick={() => (range = Math.max(300, range / 1.6))}>Closer</button>
        <button class="ghost tiny" onclick={() => (range = Math.min(9000, range * 1.6))}>Wider</button>
      </div>
      <button class="chip icon" aria-label="Close the map" onclick={onclose}><X size={16} /></button>
    </header>
    <canvas bind:this={canvas} onclick={click}></canvas>
    <p>Click to {role === 'cop' ? 'radio a spot to other units' : 'ping a spot for the crew'}.</p>
  </section>
</div>

<style>
  .backdrop {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    padding: 16px;
    background: rgba(5, 8, 14, 0.5);
    z-index: 15;
  }
  .map {
    width: min(760px, 100%);
    height: min(640px, calc(100dvh - 32px));
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  header {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
  }
  header strong {
    font-family: var(--font-sign);
    text-transform: uppercase;
    font-size: 18px;
  }
  .kinds {
    display: flex;
    gap: 4px;
  }
  header .chip {
    margin-left: auto;
  }
  .tiny {
    height: 28px;
    font-size: 12px;
    padding: 0 8px;
  }
  .tiny.on {
    border-color: var(--signal);
    color: var(--signal);
  }
  canvas {
    flex: 1;
    min-height: 0;
    width: 100%;
    border-radius: var(--radius-small);
    cursor: crosshair;
  }
  p {
    margin: 0;
    font-size: 12px;
    color: var(--text-faint);
  }
</style>
