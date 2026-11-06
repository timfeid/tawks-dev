<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import type { DriveConnection } from '../../lib/net.svelte';
  import type { HeistView, Meter } from '../../heist/protocol';

  let { conn, view }: { conn: DriveConnection; view: HeistView } = $props();

  let canvas = $state<HTMLCanvasElement>();
  let stageKey = '';
  let startedAt = 0;
  let holding = false;
  let local: Meter | null = null;
  let localAt = 0;
  let raf = 0;
  let flash = $state<string | null>(null);

  const take = $derived(view.take);
  const stage = $derived(take?.stage);

  function elapsed() {
    return performance.now() / 1000 - startedAt;
  }

  function stageChanged() {
    const key = stage ? `${stage.stage}:${'started' in stage ? stage.started : ''}` : '';
    if (key !== stageKey) {
      stageKey = key;
      startedAt = performance.now() / 1000 - (take?.elapsed ?? 0);
      holding = false;
      local = stage?.stage === 'ignition' ? { ...stage.meter, needle: stage.meter.needle, progress: 0 } : null;
      localAt = 0;
    }
  }

  $effect(() => {
    void stage;
    stageChanged();
  });

  function centre(m: Meter, t: number) {
    return 0.5 + 0.3 * Math.sin(m.freq * t + m.phase);
  }

  /** Runs the same needle the server runs, so what the player sees is what counts. */
  function runLocal(to: number) {
    if (!local) return;
    while (localAt < to) {
      const dt = Math.min(0.02, to - localAt);
      localAt += dt;
      local.needle = Math.max(0, Math.min(1, local.needle + (holding ? 0.9 : -0.7) * dt));
      if (Math.abs(local.needle - centre(local, localAt)) <= local.band) local.progress = Math.min(1, local.progress + dt / local.need);
      else local.progress = Math.max(0, local.progress - 0.08 * dt);
    }
  }

  function press() {
    if (!take?.mine || !stage) return;
    if (stage.stage === 'lock') conn.heistSend({ op: 'take_press', t: elapsed() });
  }

  function hold(on: boolean) {
    if (!take?.mine || stage?.stage !== 'ignition' || holding === on) return;
    runLocal(elapsed());
    holding = on;
    conn.heistSend({ op: 'take_hold', t: elapsed(), hold: on });
  }

  function key(e: KeyboardEvent, down: boolean) {
    if (e.code !== 'Space' || !take?.mine) return;
    e.preventDefault();
    if (down && !e.repeat) press();
    hold(down);
  }

  const kd = (e: KeyboardEvent) => key(e, true);
  const ku = (e: KeyboardEvent) => key(e, false);

  function draw() {
    raf = requestAnimationFrame(draw);
    const c = canvas;
    if (!c || !stage) return;
    const ctx = c.getContext('2d')!;
    const size = 180;
    const dpr = Math.min(2, devicePixelRatio || 1);
    if (c.width !== size * dpr) {
      c.width = c.height = size * dpr;
    }
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.clearRect(0, 0, size, size);
    const t = elapsed();
    if (stage.stage === 'lock') {
      const r = 66;
      ctx.lineWidth = 12;
      ctx.strokeStyle = 'rgba(255,255,255,0.12)';
      ctx.beginPath();
      ctx.arc(90, 90, r, 0, Math.PI * 2);
      ctx.stroke();
      const { target, window: w, speed } = stage.ring;
      ctx.strokeStyle = '#ffb547';
      ctx.beginPath();
      ctx.arc(90, 90, r, target - w - Math.PI / 2, target + w - Math.PI / 2);
      ctx.stroke();
      const a = ((speed * t) % (Math.PI * 2)) - Math.PI / 2;
      ctx.fillStyle = '#ffffff';
      ctx.beginPath();
      ctx.arc(90 + Math.cos(a) * r, 90 + Math.sin(a) * r, 8, 0, Math.PI * 2);
      ctx.fill();
      for (let i = 0; i < 3; i++) {
        ctx.fillStyle = i < stage.hits ? '#3fd08f' : 'rgba(255,255,255,0.2)';
        ctx.beginPath();
        ctx.arc(70 + i * 20, 90, 6, 0, Math.PI * 2);
        ctx.fill();
      }
    } else if (stage.stage === 'ignition' && local) {
      runLocal(t);
      const x0 = 30;
      const h = 140;
      ctx.fillStyle = 'rgba(255,255,255,0.08)';
      ctx.fillRect(x0, 20, 26, h);
      const c0 = centre(local, localAt);
      ctx.fillStyle = 'rgba(255,181,71,0.45)';
      ctx.fillRect(x0, 20 + h * (1 - c0 - local.band), 26, h * local.band * 2);
      ctx.fillStyle = '#ffffff';
      ctx.fillRect(x0 - 6, 20 + h * (1 - local.needle) - 2, 38, 4);
      ctx.fillStyle = 'rgba(255,255,255,0.1)';
      ctx.fillRect(80, 150, 80, 10);
      ctx.fillStyle = '#3fd08f';
      // A lookout speeds the server's start, so its progress can run ahead of this one.
      ctx.fillRect(80, 150, 80 * Math.max(local.progress, stage.meter.progress), 10);
    }
  }

  onMount(() => {
    window.addEventListener('keydown', kd);
    window.addEventListener('keyup', ku);
    raf = requestAnimationFrame(draw);
  });
  onDestroy(() => {
    window.removeEventListener('keydown', kd);
    window.removeEventListener('keyup', ku);
    cancelAnimationFrame(raf);
  });

  function choose(entry: 'handle' | 'lock' | 'window') {
    conn.heistSend({ op: 'take_choose', entry });
    flash = null;
  }
</script>

{#if take}
  <section class="panel take" aria-label="Taking the car">
    {#if !take.mine}
      <p>A crewmate is at a car. Stand close to keep a lookout and speed them up.</p>
    {:else if stage?.stage === 'door'}
      <h2>Get in</h2>
      <div class="choices">
        <button class="ghost" disabled={stage.tried_handle} onclick={() => choose('handle')}><strong>Try the handle</strong><span>{stage.tried_handle ? 'Locked' : 'Silent, if your luck holds'}</span></button>
        <button class="ghost" onclick={() => choose('lock')}><strong>Work the lock</strong><span>Quiet but slow. Hit the mark three times</span></button>
        <button class="ghost" onclick={() => choose('window')}><strong>Break a window</strong><span>Fast and loud. Alarms, and the car shows it</span></button>
      </div>
      <button class="ghost small" onclick={() => conn.heistSend({ op: 'take_abandon' })}>Walk away</button>
    {:else if stage?.stage === 'lock'}
      <h2>Work the lock</h2>
      <canvas bind:this={canvas} style:width="180px" style:height="180px"></canvas>
      <p>Press Space when the dot is over the mark. A miss makes noise.</p>
    {:else if stage?.stage === 'ignition'}
      <h2>Start it</h2>
      <canvas bind:this={canvas} style:width="180px" style:height="180px"></canvas>
      <p>Hold Space to raise the needle, let go to drop it. Keep it in the band until the bar fills.</p>
      <div class="touch-row">
        <button class="ghost" onpointerdown={() => hold(true)} onpointerup={() => hold(false)} onpointerleave={() => hold(false)}>Hold</button>
      </div>
    {/if}
    {#if take.alarm}<p class="alarm">The alarm is going. Ride it out or walk away.</p>{/if}
    {#if stage?.stage === 'lock'}<div class="touch-row"><button class="ghost" onpointerdown={press}>Press</button></div>{/if}
  </section>
{/if}

<style>
  .take {
    position: absolute;
    right: 16px;
    bottom: 90px;
    width: min(320px, calc(100% - 32px));
    padding: 14px;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    z-index: 9;
    text-align: center;
  }
  h2 {
    margin: 0;
    font-family: var(--font-sign);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  p {
    margin: 0;
    color: var(--text-dim);
    font-size: 13px;
  }
  .choices {
    display: flex;
    flex-direction: column;
    gap: 6px;
    width: 100%;
  }
  .choices button {
    height: auto;
    padding: 8px 10px;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    text-align: left;
  }
  .choices span {
    font-size: 12px;
    color: var(--text-faint);
    font-weight: 500;
  }
  .small {
    height: 30px;
    font-size: 13px;
  }
  .alarm {
    color: #ffb547;
    font-weight: 700;
  }
  .touch-row button {
    width: 120px;
  }
</style>
