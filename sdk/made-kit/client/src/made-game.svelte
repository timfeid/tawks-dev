<script lang="ts">
  import { onMount } from 'svelte';
  import { LoaderCircle } from '@lucide/svelte';
  import type { EndReason, Hud, LogLine, Meta, Placements, Seat, Welcome } from './contract';
  import type { MadeSource, SourceStatus } from './connection';
  import { MadeRenderer } from './renderer';
  import { MadeControls, sameInput, type FullInput } from './controls';
  import { SceneBuffer } from './scene-buffer';
  import { SnapshotClock } from './party/snapshot-clock';
  import MadeHud from './made-hud.svelte';
  import MadePodium from './made-podium.svelte';

  let {
    source,
    parked = false,
    onwelcome,
    onend,
    onlog,
  }: {
    source: MadeSource;
    /** Hidden but kept connected, as when the game is shown somewhere else. */
    parked?: boolean;
    onwelcome?: (welcome: Welcome) => void;
    onend?: (placements: Placements, reason: EndReason) => void;
    onlog?: (line: LogLine) => void;
  } = $props();

  let root: HTMLDivElement;
  let canvas: HTMLCanvasElement;
  let renderer = $state<MadeRenderer | null>(null);
  let status = $state<SourceStatus>('connecting');
  let meta = $state<Meta | null>(null);
  let roster = $state<Seat[]>([]);
  let mySeat = $state<number | null>(null);
  let hud = $state<Hud | null>(null);
  let placements = $state<Placements | null>(null);
  let endReason = $state<EndReason | null>(null);
  let error = $state<string | null>(null);
  let briefUntil = $state(0);
  let now = $state(0);
  let muted = $state(false);
  let fullscreen = $state(false);
  let hasFrame = $state(false);

  const BRIEF_SECONDS = 5;
  const ERROR_SECONDS = 5;
  const KEEPALIVE_MS = 250;
  const HUD_EVERY_MS = 100;

  onMount(() => {
    const mobile = window.matchMedia('(pointer: coarse)').matches || window.innerWidth < 760;
    const view = new MadeRenderer(canvas, { mobile, sounds: true });
    renderer = view;
    muted = view.muted;
    const buffer = new SceneBuffer();
    const clock = new SnapshotClock();
    const controls = new MadeControls(root, {
      ground: (x, y) => view.groundPoint(x, y),
      me: () => (mySeat === null ? null : view.seatPosition(mySeat)),
    });
    let lastT = -1;
    let lastSent: FullInput | null = null;
    let lastSentAt = 0;
    let lastHudAt = 0;

    const stopMessages = source.subscribe((message) => {
      const seconds = performance.now() / 1000;
      switch (message.type) {
        case 'welcome': {
          const welcome = message.welcome;
          mySeat = welcome.seat;
          meta = welcome.meta;
          roster = welcome.roster;
          view.mySeat = welcome.seat;
          view.setRoster(welcome.roster);
          controls.enabled = welcome.seat !== null;
          briefUntil = seconds + BRIEF_SECONDS;
          error = null;
          onwelcome?.(welcome);
          break;
        }
        case 'roster':
          roster = message.roster;
          view.setRoster(message.roster);
          break;
        case 'restarting':
          startOver(seconds);
          break;
        case 'frame': {
          const frame = message.frame;
          if (frame.base === null && frame.t < lastT) startOver(seconds);
          if (buffer.push(frame)) {
            clock.arrive(frame.t, seconds);
            lastT = frame.t;
            hasFrame = true;
          } else source.send({ type: 'resync' });
          break;
        }
        case 'end':
          placements = message.placements;
          endReason = message.reason;
          view.showPodium(message.placements);
          onend?.(message.placements, message.reason);
          break;
        case 'log':
          onlog?.(message.line);
          break;
        case 'error':
          // A code such as pass_expired is handled by fetching a new pass, not shown.
          if (!/^[a-z_]+$/.test(message.message)) {
            const shown = message.message;
            error = shown;
            setTimeout(() => error === shown && (error = null), ERROR_SECONDS * 1000);
          }
          break;
      }
    });
    function startOver(seconds: number) {
      clock.reset();
      buffer.reset();
      lastT = -1;
      if (placements) {
        placements = null;
        endReason = null;
        view.showPodium(null);
      }
      briefUntil = seconds + BRIEF_SECONDS;
    }
    const stopStatus = source.onStatus((next) => (status = next));

    const resize = () => {
      const rect = canvas.parentElement!.getBoundingClientRect();
      view.resize(Math.max(1, Math.round(rect.width)), Math.max(1, Math.round(rect.height)));
    };
    const observer = new ResizeObserver(resize);
    observer.observe(canvas.parentElement!);
    resize();

    let frame = 0;
    let last = performance.now();
    const loop = (time: number) => {
      frame = requestAnimationFrame(loop);
      const dt = Math.min(0.1, (time - last) / 1000);
      last = time;
      const seconds = time / 1000;
      const renderTime = clock.renderTime(seconds);
      const sample = renderTime === null ? null : buffer.sample(renderTime);
      if (renderTime !== null) view.play(buffer.takeEffects(renderTime));
      view.draw(sample, seconds, dt);
      if (sample && time - lastHudAt > HUD_EVERY_MS) {
        hud = sample.state.hud;
        now = seconds;
        lastHudAt = time;
      }
      const input = controls.read();
      if (controls.enabled && !placements && (!sameInput(lastSent, input) || time - lastSentAt > KEEPALIVE_MS)) {
        if (source.send({ type: 'input', input })) {
          controls.sent();
          lastSent = input;
          lastSentAt = time;
        }
      }
    };
    frame = requestAnimationFrame(loop);

    const onFullscreen = () => (fullscreen = document.fullscreenElement === root);
    document.addEventListener('fullscreenchange', onFullscreen);

    return () => {
      cancelAnimationFrame(frame);
      observer.disconnect();
      document.removeEventListener('fullscreenchange', onFullscreen);
      stopMessages();
      stopStatus();
      controls.dispose();
      view.dispose();
      renderer = null;
    };
  });

  $effect(() => {
    renderer?.setSuspended(parked);
  });

  function toggleMute() {
    if (!renderer) return;
    renderer.setMuted(!renderer.muted);
    muted = renderer.muted;
  }

  function toggleFullscreen() {
    if (document.fullscreenElement === root) void document.exitFullscreen();
    else void root.requestFullscreen?.().catch(() => {});
  }

  const colorOf = (seat: number) => renderer?.seatColor(seat) ?? '#ffd23f';
</script>

<div
  bind:this={root}
  class="made-game relative h-full min-h-72 w-full overflow-hidden bg-sky-300"
  data-made-game
  data-status={status}
  data-seat={mySeat ?? ''}
  role="application"
  aria-label={meta?.name ?? 'Minigame'}
  tabindex="-1"
>
  <canvas bind:this={canvas} class="absolute inset-0 block h-full w-full touch-none outline-none"></canvas>
  <MadeHud
    {hud}
    {meta}
    {roster}
    {mySeat}
    {status}
    {colorOf}
    showBrief={!placements && now < briefUntil}
    ended={!!placements}
    {muted}
    {fullscreen}
    ontogglemute={toggleMute}
    ontogglefullscreen={toggleFullscreen}
  />
  {#if placements}
    <MadePodium {placements} reason={endReason} {roster} {mySeat} {colorOf} />
  {/if}
  {#if !hasFrame && !error}
    <div class="absolute inset-0 grid place-items-center">
      <div class="flex items-center gap-2 rounded-lg bg-slate-950/70 px-4 py-2 text-sm text-white" role="status">
        <LoaderCircle class="size-4 animate-spin" />
        {status === 'offline' ? 'Reconnecting' : 'Joining the game'}
      </div>
    </div>
  {/if}
  {#if error}
    <div class="absolute inset-x-0 bottom-16 flex justify-center">
      <div class="rounded-lg bg-rose-950/85 px-4 py-2 text-sm text-rose-100" role="alert">{error}</div>
    </div>
  {/if}
</div>
