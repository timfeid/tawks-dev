<script lang="ts">
  import { onDestroy } from 'svelte';
  import { audioContext, heardAt, scheduleClicks, unlockAudio } from '../lib/audio';
  import { Engine, HIT } from '../lib/engine';
  import { Controls, type Bindings, type Layout } from '../lib/input';
  import { drawHighway } from '../lib/render';
  import type { Note } from '../lib/types';

  let {
    layout,
    bindings,
    lookahead,
    inputOffset = $bindable(),
    videoOffset = $bindable(),
  }: {
    layout: Layout;
    bindings: Bindings;
    lookahead: number;
    inputOffset: number;
    videoOffset: number;
  } = $props();

  /** 90 BPM, slow enough to land every note on purpose. */
  const INTERVAL_MS = 667;
  const NOTES = 24;
  const LEAD_MS = 2500;

  let canvas = $state<HTMLCanvasElement>();
  let running = $state(false);
  let deviations = $state<number[]>([]);
  let finished = $state(false);

  let engine: Engine | null = null;
  let controls: Controls | null = null;
  let zero = 0;
  let raf = 0;
  let counted = new Set<number>();

  const lane = layout === 'drums' ? 1 : 0;
  const median = $derived.by(() => {
    if (!deviations.length) return null;
    const sorted = [...deviations].sort((a, b) => a - b);
    return Math.round(sorted[Math.floor(sorted.length / 2)]);
  });

  function songTime(performanceTime: number) {
    return (heardAt(audioContext(), performanceTime) - zero) * 1000;
  }

  function start() {
    unlockAudio();
    stop();
    deviations = [];
    finished = false;
    counted = new Set();
    const notes: Note[] = Array.from({ length: NOTES }, (_, i) => ({
      t: i * INTERVAL_MS,
      len: 0,
      lanes: 1 << lane,
      kind: 'strum',
    }));
    engine = new Engine(
      { instrument: layout === 'drums' ? 'drums' : 'guitar', difficulty: 'expert', notes, star_power: [] },
      notes.map((n) => ({ t: n.t })),
    );
    zero = scheduleClicks(NOTES, INTERVAL_MS, LEAD_MS)[0];
    running = true;

    // Guitars are timed by the strum and drums by the pad, the same moments the game judges.
    const timed = layout === 'drums' ? ['lane0', 'lane1', 'lane2', 'lane3', 'lane4'] : ['strum_up', 'strum_down'];
    controls = new Controls(bindings, (control, down, at) => {
      if (!down || !engine) return;
      if (layout === 'guitar' && control.startsWith('lane')) {
        engine.held |= 1 << Number(control.slice(4));
        return;
      }
      if (!timed.includes(control)) return;
      const raw = songTime(at);
      const index = Math.round(raw / INTERVAL_MS);
      if (index < 0 || index >= NOTES || counted.has(index)) return;
      const deviation = raw - index * INTERVAL_MS;
      if (Math.abs(deviation) > INTERVAL_MS / 2) return;
      counted.add(index);
      deviations = [...deviations, deviation];
      engine.judged[index] = HIT;
      engine.flashes[lane] = raw - inputOffset;
    });
    controls.start();
    raf = requestAnimationFrame(frame);
  }

  function stop() {
    cancelAnimationFrame(raf);
    controls?.stop();
    controls = null;
    running = false;
  }

  function frame() {
    raf = requestAnimationFrame(frame);
    if (!engine || !canvas) return;
    const now = songTime(performance.now());
    engine.update(now - 200);
    if (layout === 'guitar') engine.held = 0;
    const dpr = window.devicePixelRatio || 1;
    const width = canvas.clientWidth;
    const height = canvas.clientHeight;
    if (canvas.width !== Math.round(width * dpr)) canvas.width = Math.round(width * dpr);
    if (canvas.height !== Math.round(height * dpr)) canvas.height = Math.round(height * dpr);
    const c = canvas.getContext('2d')!;
    c.setTransform(dpr, 0, 0, dpr, 0, 0);
    drawHighway(c, width, height, { engine, beats: [], time: now - videoOffset, lookahead });
    if (now > (NOTES - 1) * INTERVAL_MS + 1200) {
      stop();
      finished = true;
    }
  }

  function apply() {
    if (median !== null) inputOffset = median;
  }

  onDestroy(stop);

  const describe = (ms: number) => `${Math.abs(Math.round(ms))} ms ${ms >= 0 ? 'late' : 'early'}`;
</script>

<div class="space-y-3">
  <div class="relative h-72 overflow-hidden rounded-lg border border-white/10 bg-black">
    <canvas bind:this={canvas} class="absolute inset-0 h-full w-full"></canvas>
    {#if !running}
      <div class="absolute inset-0 flex flex-col items-center justify-center gap-3 bg-black/70 p-6 text-center">
        <p class="max-w-sm text-sm text-neutral-300">
          {layout === 'drums'
            ? 'Hit any pad as each note crosses the line, in time with the click.'
            : 'Strum as each note crosses the line, in time with the click.'}
          Go by the sound, not the picture.
        </p>
        <button
          class="rounded-lg bg-primary px-4 py-1.5 text-sm font-semibold text-primary-foreground hover:bg-primary/90"
          onclick={start}
        >
          {finished ? 'Go again' : 'Start'}
        </button>
      </div>
    {/if}
  </div>

  <div class="flex flex-wrap gap-1">
    {#each deviations.slice(-12) as deviation, i (i)}
      <span
        class="rounded px-1.5 py-0.5 font-mono text-[11px] {Math.abs(deviation - inputOffset) <= 35
          ? 'bg-primary/15 text-primary'
          : 'bg-white/10 text-neutral-300'}">{deviation > 0 ? '+' : ''}{Math.round(deviation)}</span
      >
    {/each}
  </div>

  {#if median !== null}
    <div class="flex items-center gap-3 text-sm">
      <span class="text-neutral-300">
        Your hits land <span class="font-mono text-white">{describe(median)}</span> over {deviations.length} notes.
      </span>
      {#if median !== inputOffset}
        <button
          class="ml-auto rounded-lg bg-white px-3 py-1 text-xs font-semibold text-black hover:bg-neutral-200"
          onclick={apply}
        >
          Use {median} ms
        </button>
      {:else}
        <span class="ml-auto text-xs text-primary">In use</span>
      {/if}
    </div>
  {/if}
</div>
