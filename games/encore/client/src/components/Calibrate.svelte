<script lang="ts">
  import { onDestroy } from 'svelte';
  import { audioContext, heardAt, reportedLatencyMs, scheduleClicks, unlockAudio, watchOutputDevice } from '../lib/audio';
  import { MIN_CALIBRATION_TAPS, calibrationResult, videoResult } from '../lib/calibration';
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

  type Mode = 'audio' | 'video';

  /** 90 BPM, slow enough to land every tap on purpose. */
  const INTERVAL_MS = 667;
  const NOTES = 24;
  const LEAD_MS = 2500;

  let canvas = $state<HTMLCanvasElement>();
  let mode = $state<Mode>('audio');
  let running = $state<Mode | null>(null);
  let deviations = $state<number[]>([]);
  let finishedMode = $state<Mode | null>(null);
  let tapped = $state(0);
  let latency = $state<number | null>(reportedLatencyMs());

  let engine: Engine | null = null;
  let controls: Controls | null = null;
  let zero = 0;
  let raf = 0;
  let counted = new Set<number>();

  const lane = layout === 'drums' ? 1 : 0;
  /** The offset this run measured: audio taps against the click, video taps against the note. */
  const measured = $derived.by(() => {
    if (finishedMode === 'audio') return calibrationResult(deviations);
    if (finishedMode === 'video') return videoResult(videoOffset, deviations);
    return null;
  });
  const current = $derived(finishedMode === 'video' ? videoOffset : inputOffset);

  const stopWatching = watchOutputDevice(() => (latency = reportedLatencyMs()));

  function songTime(performanceTime: number) {
    return (heardAt(audioContext(), performanceTime) - zero) * 1000;
  }

  function start() {
    unlockAudio();
    stop();
    deviations = [];
    finishedMode = null;
    tapped = 0;
    counted = new Set();
    const run = mode;
    const notes: Note[] = Array.from({ length: NOTES }, (_, i) => ({ t: i * INTERVAL_MS, len: 0, lanes: 1 << lane, kind: 'strum' }));
    engine = new Engine(
      { instrument: layout === 'drums' ? 'drums' : 'guitar', difficulty: 'expert', notes, star_power: [] },
      notes.map((n) => ({ t: n.t })),
    );
    if (run === 'audio') zero = scheduleClicks(NOTES, INTERVAL_MS, LEAD_MS)[0];
    else zero = heardAt(audioContext(), performance.now()) + LEAD_MS / 1000;
    running = run;

    // Guitars are timed by the strum and drums by the pad, the same moments the game judges.
    const timed = layout === 'drums' ? ['lane0', 'lane1', 'lane2', 'lane3', 'lane4'] : ['strum_up', 'strum_down'];
    controls = new Controls(bindings, (control, down, at) => {
      if (!down || !engine || !timed.includes(control)) return;
      const raw = songTime(at);
      // Audio taps measure the whole delay from sound to press; video taps are judged as the game would.
      const time = run === 'audio' ? raw : raw - inputOffset;
      const index = Math.round(time / INTERVAL_MS);
      if (index < 0 || index >= NOTES || counted.has(index)) return;
      const deviation = time - index * INTERVAL_MS;
      if (Math.abs(deviation) > INTERVAL_MS / 2) return;
      counted.add(index);
      tapped = counted.size;
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
    running = null;
  }

  function frame() {
    raf = requestAnimationFrame(frame);
    if (!engine || !canvas || !running) return;
    latency = reportedLatencyMs();
    const now = songTime(performance.now());
    const dpr = window.devicePixelRatio || 1;
    const width = canvas.clientWidth;
    const height = canvas.clientHeight;
    if (canvas.width !== Math.round(width * dpr)) canvas.width = Math.round(width * dpr);
    if (canvas.height !== Math.round(height * dpr)) canvas.height = Math.round(height * dpr);
    const c = canvas.getContext('2d')!;
    c.setTransform(dpr, 0, 0, dpr, 0, 0);
    if (running === 'video') {
      engine.update(now - inputOffset - 200);
      drawHighway(c, width, height, { engine, beats: [], time: now - videoOffset, lookahead });
    } else {
      c.clearRect(0, 0, width, height);
    }
    if (now > (NOTES - 1) * INTERVAL_MS + 1200) {
      finishedMode = running;
      stop();
    }
  }

  function apply() {
    if (measured === null) return;
    if (finishedMode === 'audio') inputOffset = measured;
    else videoOffset = measured;
  }

  onDestroy(() => {
    stop();
    stopWatching();
  });

  const describe = (ms: number) => `${Math.abs(Math.round(ms))} ms ${ms >= 0 ? 'late' : 'early'}`;
</script>

<div class="space-y-3">
  <div class="flex items-center gap-2">
    {#each [['audio', 'Audio: tap to the click'], ['video', 'Video: tap to the notes']] as const as [value, label] (value)}
      <button
        class="rounded-full border px-3 py-1 text-xs {mode === value
          ? 'border-primary bg-primary/15'
          : 'border-white/10 text-neutral-400 hover:text-white'}"
        disabled={!!running}
        onclick={() => (mode = value)}
      >
        {label}
      </button>
    {/each}
    <span class="ml-auto text-xs text-neutral-500">
      {latency === null ? 'Your browser reports no output delay.' : `Output delay of ${latency} ms is already allowed for.`}
    </span>
  </div>

  <div class="relative h-72 overflow-hidden rounded-lg border border-white/10 bg-black">
    <canvas bind:this={canvas} class="absolute inset-0 h-full w-full"></canvas>
    {#if running === 'audio'}
      <div class="absolute inset-0 flex flex-col items-center justify-center gap-2 text-center">
        <p class="text-sm text-neutral-300">Close your eyes and {layout === 'drums' ? 'hit a pad' : 'strum'} on every click.</p>
        <p class="font-mono text-2xl text-white">{tapped} / {NOTES}</p>
      </div>
    {:else if !running}
      <div class="absolute inset-0 flex flex-col items-center justify-center gap-3 bg-black/70 p-6 text-center">
        <p class="max-w-sm text-sm text-neutral-300">
          {#if mode === 'audio'}
            {layout === 'drums' ? 'Hit any pad' : 'Strum'} on each click you hear. This sets the audio offset, which moves when your hits are
            judged.
          {:else}
            {layout === 'drums' ? 'Hit any pad' : 'Strum'} as each note reaches the line. There is no sound. This sets the video offset,
            which only moves the notes on screen. Set the audio offset first.
          {/if}
        </p>
        <button
          class="rounded-lg bg-primary px-4 py-1.5 text-sm font-semibold text-primary-foreground hover:bg-primary/90"
          onclick={start}
        >
          {finishedMode === mode ? 'Go again' : 'Start'}
        </button>
      </div>
    {/if}
  </div>

  <div class="flex flex-wrap gap-1">
    {#each deviations.slice(-12) as deviation, i (i)}
      <span class="rounded bg-white/10 px-1.5 py-0.5 font-mono text-[11px] text-neutral-300"
        >{deviation > 0 ? '+' : ''}{Math.round(deviation)}</span
      >
    {/each}
  </div>

  {#if finishedMode}
    <div class="flex items-center gap-3 text-sm">
      {#if measured === null}
        <span class="text-neutral-300">
          Only {deviations.length} steady taps; at least {MIN_CALIBRATION_TAPS} are needed. Try again.
        </span>
      {:else}
        <span class="text-neutral-300">
          {#if finishedMode === 'audio'}
            Your hits land <span class="font-mono text-white">{describe(measured)}</span> after the click.
          {:else}
            Video offset to line up the notes: <span class="font-mono text-white">{measured} ms</span>.
          {/if}
        </span>
        {#if measured !== current}
          <button
            class="ml-auto rounded-lg bg-white px-3 py-1 text-xs font-semibold text-black hover:bg-neutral-200"
            onclick={apply}
          >
            Use {measured} ms
          </button>
        {:else}
          <span class="ml-auto text-xs text-primary">In use</span>
        {/if}
      {/if}
    </div>
  {/if}
</div>
