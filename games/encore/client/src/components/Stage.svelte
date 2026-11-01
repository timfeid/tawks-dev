<script lang="ts">
  import { onDestroy, onMount, untrack } from 'svelte';
  import { X } from '@lucide/svelte';
  import { SongPlayer, playScratch } from '../lib/audio';
  import { Engine, type Action } from '../lib/engine';
  import { Controls, type Control, type Layout } from '../lib/input';
  import { drawHighway } from '../lib/render';
  import { drawVocals, pitchRange } from '../lib/render-vocals';
  import { VocalEngine, hzToMidi, type VoiceSample } from '../lib/vocals';
  import { listen, type HeardSample } from '../lib/voice';
  import type { MicrophoneState } from '@tawks/activity-sdk';
  import type { RoomConnection } from '../lib/room.svelte';
  import type { Settings } from '../lib/settings';
  import { DIFFICULTIES, INSTRUMENT_LABELS, type Chart, type Instrument, type PartChoice, type Track } from '../lib/types';
  import BandList from './BandList.svelte';
  import Lyrics from './Lyrics.svelte';

  let { conn, settings }: { conn: RoomConnection; settings: Settings } = $props();

  const PROGRESS_EVERY_MS = 200;
  /** For testing: `?autoplay` plays every note perfectly. */
  const autoplay = new URLSearchParams(location.search).has('autoplay');
  let autoIndex = 0;

  const song = untrack(() => conn.room!.song!);
  const part = untrack(() => conn.me!.part!);
  const layout: Layout = part.instrument === 'drums' ? 'drums' : 'guitar';
  const singing = part.instrument === 'vocals';
  const room = $derived(conn.room!);

  let status = $state<'loading' | 'ready' | 'failed'>('loading');
  let loaded = $state(0);
  let failure = $state<string | null>(null);
  let canvas = $state<HTMLCanvasElement>();
  let lyricTime = $state(-1e9);
  let hud = $state({ score: 0, multiplier: 1, streak: 0, star: 0, starActive: false, countdown: 0, section: '', done: false, missing: false });

  let engine: Engine | null = null;
  let vocal: VocalEngine | null = null;
  let range = { low: 55, high: 72 };
  let voice: { state: MicrophoneState; stop: () => void } | null = null;
  let micState = $state<MicrophoneState | null>(null);
  const heardQueue: HeardSample[] = [];
  let player: SongPlayer | null = null;
  let controls: Controls | null = null;
  let chart = $state.raw<Chart | null>(null);
  let started = false;
  let raf = 0;
  let lastSent = 0;
  let sentDone = false;
  let lastOverstrum = -1e9;
  let songEnd = 0;
  const queue: { control: Control; down: boolean; at: number }[] = [];
  const missing = new Map<Instrument, boolean>();

  /** The chosen difficulty, or the nearest one the chart has for this instrument. */
  function pickTrack(chart: Chart, part: PartChoice): Track | null {
    const tracks = chart.tracks.filter((t) => t.instrument === part.instrument);
    const want = DIFFICULTIES.indexOf(part.difficulty);
    tracks.sort((a, b) => Math.abs(DIFFICULTIES.indexOf(a.difficulty) - want) - Math.abs(DIFFICULTIES.indexOf(b.difficulty) - want));
    return tracks[0] ?? null;
  }

  onMount(async () => {
    const pass = conn.current;
    const query = `token=${encodeURIComponent(pass.token)}`;
    try {
      const response = await fetch(`${pass.api_url}/songs/${song.id}/chart?${query}`);
      if (!response.ok) throw new Error('The chart did not load');
      chart = (await response.json()) as Chart;
      if (singing) {
        if (!chart.vocals?.length) throw new Error('This chart has no vocal part');
        vocal = new VocalEngine(chart.vocals, part.difficulty);
        range = pitchRange(vocal);
      } else {
        const track = pickTrack(chart, part);
        if (!track) throw new Error(`This chart has no ${INSTRUMENT_LABELS[part.instrument].toLowerCase()} part`);
        engine = new Engine(track, chart.beats);
      }
      songEnd = Math.max((engine ?? vocal)!.end, song.length_ms, chart.length) + 500;
      player = new SongPlayer(settings.volume);
      const band = untrack(() => room.players.filter((p) => p.playing && p.part).map((p) => p.part!.instrument));
      await player.load(
        (stem) => `${pass.api_url}/songs/${song.id}/files/${stem}?${query}`,
        song,
        [...new Set(band)],
        (done, total) => (loaded = total ? done / total : 1),
      );
      status = 'ready';
      if (singing) void startMic();
      else {
        controls = new Controls(settings.bindings[layout], (control, down, at) => queue.push({ control, down, at }));
        controls.start();
      }
      raf = requestAnimationFrame(frame);
    } catch (error) {
      failure = (error as Error).message;
      status = 'failed';
      conn.send({ type: 'progress', progress: emptyProgress(), done: true });
    } finally {
      conn.send({ type: 'loaded' });
    }
  });

  async function startMic() {
    if (autoplay) return;
    voice?.stop();
    voice = await listen((samples) => heardQueue.push(...samples));
    micState = voice.state;
  }

  onDestroy(() => {
    cancelAnimationFrame(raf);
    controls?.stop();
    voice?.stop();
    player?.dispose();
  });

  $effect(() => {
    const startAt = room.start_at;
    if (status === 'ready' && startAt && !started && player) {
      started = true;
      player.start(startAt - conn.serverNow());
    }
  });

  $effect(() => {
    if (status === 'failed' && room.phase === 'playing' && !sentDone) {
      sentDone = true;
      conn.send({ type: 'progress', progress: emptyProgress(), done: true });
    }
  });

  function emptyProgress() {
    return { score: 0, streak: 0, best_streak: 0, multiplier: 1, hit: 0, missed: 0, total: 0, star_meter: 0, star_active: false, muted: false };
  }

  function applyMissing() {
    const scorer = engine ?? vocal;
    if (!player || !scorer) return;
    const now = new Map<Instrument, boolean>();
    for (const p of room.players) {
      if (!p.playing || !p.part) continue;
      const out = p.conn === conn.conn ? scorer.muted : p.progress.muted;
      now.set(p.part.instrument, (now.get(p.part.instrument) ?? true) && out);
    }
    for (const [instrument, out] of now) {
      if (missing.get(instrument) === out) continue;
      missing.set(instrument, out);
      player.setMissing(instrument, out);
    }
  }

  function playAhead(engine: Engine, until: number) {
    for (; autoIndex < engine.notes.length && engine.notes[autoIndex].t <= until; autoIndex++) {
      const note = engine.notes[autoIndex];
      if (engine.drums) {
        const lane = 31 - Math.clz32(note.lanes);
        engine.input({ action: `lane${lane}` as Action, down: true, time: note.t });
        continue;
      }
      for (let lane = 0; lane < 5; lane++) {
        const down = !!(note.lanes & (1 << lane));
        if (!!(engine.held & (1 << lane)) !== down) engine.input({ action: `lane${lane}` as Action, down, time: note.t - 1 });
      }
      engine.input({ action: 'strum', down: true, time: note.t });
    }
  }

  /** For autoplay tests, sings each note on pitch. */
  function singAhead(vocal: VocalEngine, from: number, to: number) {
    const samples: VoiceSample[] = [];
    for (let t = Math.max(from, 0); t < to; t += 16) {
      const i = vocal.noteAt(t);
      samples.push({ time: t, midi: i >= 0 ? vocal.notes[i].pitch : null, level: i >= 0 ? 0.1 : 0 });
    }
    vocal.feed(samples);
  }

  let lastSung = 0;

  function frameVocals(now: number) {
    if (!vocal || !player || !canvas) return;
    let heard = -3000;
    if (started) {
      heard = player.songTime(now);
      if (autoplay) singAhead(vocal, lastSung, heard);
      lastSung = heard;
      const samples = heardQueue.splice(0).map((s) => ({
        time: player!.songTime(s.at),
        midi: s.hz === null ? null : hzToMidi(s.hz),
        level: s.level,
      }));
      vocal.feed(samples);
      vocal.update(heard);
      applyMissing();
      const done = heard > songEnd;
      if ((now - lastSent > PROGRESS_EVERY_MS && !sentDone) || (done && !sentDone)) {
        lastSent = now;
        sentDone = done;
        conn.send({ type: 'progress', progress: vocal.progress(), done });
      }
    } else heardQueue.length = 0;
    const c = sizeCanvas(canvas);
    drawVocals(c, canvas.clientWidth, canvas.clientHeight, vocal, heard - settings.videoOffset, range, '#39ff14');
    updateHud(vocal, heard);
  }

  function sizeCanvas(canvas: HTMLCanvasElement) {
    const dpr = window.devicePixelRatio || 1;
    const width = canvas.clientWidth;
    const height = canvas.clientHeight;
    if (canvas.width !== Math.round(width * dpr) || canvas.height !== Math.round(height * dpr)) {
      canvas.width = Math.round(width * dpr);
      canvas.height = Math.round(height * dpr);
    }
    const c = canvas.getContext('2d')!;
    c.setTransform(dpr, 0, 0, dpr, 0, 0);
    return c;
  }

  function updateHud(scorer: Engine | VocalEngine, heard: number) {
    lyricTime = heard;
    const section = chart!.sections.findLast((s) => s.t <= heard)?.name ?? '';
    hud = {
      score: scorer.score,
      multiplier: scorer.multiplier,
      streak: scorer.streak,
      star: scorer.starMeter,
      starActive: scorer.starActive,
      countdown: started && heard < 0 ? Math.ceil(-heard / 1000) : 0,
      missing: heard - scorer.lastMissAt < 400,
      section,
      done: sentDone,
    };
  }

  function frame() {
    raf = requestAnimationFrame(frame);
    const now = performance.now();
    if (singing) return frameVocals(now);
    if (!engine || !player || !canvas) return;
    let heard = -3000;
    if (started) {
      queue.sort((a, b) => a.at - b.at);
      for (const event of queue.splice(0)) {
        const action: Action = event.control === 'strum_up' || event.control === 'strum_down' ? 'strum' : event.control;
        engine.input({ action, down: event.down, time: player.songTime(event.at) - settings.inputOffset });
      }
      heard = player.songTime(now);
      const judged = heard - settings.inputOffset;
      if (autoplay) playAhead(engine, judged);
      engine.update(judged);
      if (engine.overstrumAt !== lastOverstrum) {
        lastOverstrum = engine.overstrumAt;
        if (judged > 0) playScratch(settings.volume * 0.6);
      }
      applyMissing();
      const done = heard > songEnd;
      if ((now - lastSent > PROGRESS_EVERY_MS && !sentDone) || (done && !sentDone)) {
        lastSent = now;
        sentDone = done;
        conn.send({ type: 'progress', progress: engine.progress(), done });
      }
    } else {
      for (const event of queue.splice(0)) {
        const action: Action = event.control === 'strum_up' || event.control === 'strum_down' ? 'strum' : event.control;
        if (action !== 'strum') engine.input({ action, down: event.down, time: -1 });
      }
    }

    const c = sizeCanvas(canvas);
    drawHighway(c, canvas.clientWidth, canvas.clientHeight, {
      engine,
      beats: chart!.beats,
      time: heard - settings.videoOffset,
      lookahead: settings.lookahead,
    });
    updateHud(engine, heard);
  }
</script>

<div class="relative h-full w-full overflow-hidden bg-[radial-gradient(ellipse_at_bottom,rgba(57,255,20,0.07)_0%,#000_70%)]">
  <canvas bind:this={canvas} class="absolute inset-0 h-full w-full"></canvas>

  {#if status === 'loading'}
    <div class="absolute inset-0 flex flex-col items-center justify-center gap-3 bg-black/70">
      <div class="text-sm text-neutral-300">Loading {song.name}...</div>
      <div class="h-1.5 w-64 overflow-hidden rounded-full bg-white/10">
        <div class="h-full bg-primary transition-[width]" style="width: {Math.round(loaded * 100)}%"></div>
      </div>
    </div>
  {:else if status === 'failed'}
    <div class="absolute inset-0 flex items-center justify-center bg-black/80 p-6 text-center text-sm text-neutral-300">
      {failure}. You can join the next song.
    </div>
  {:else if !room.start_at}
    <div class="absolute inset-0 flex items-center justify-center text-sm text-neutral-300">Waiting for the band to load...</div>
  {/if}

  {#if chart?.lyrics?.length && status === 'ready'}
    <Lyrics lines={chart.lyrics} time={lyricTime} top={singing ? '56%' : '4rem'} />
  {/if}

  {#if singing && micState && micState !== 'on'}
    <div class="absolute top-[56%] left-1/2 mt-24 flex -translate-x-1/2 items-center gap-3 rounded-lg border border-red-400/30 bg-black/80 px-4 py-2 text-sm text-neutral-200">
      {micState === 'denied' ? 'Encore cannot hear your microphone.' : 'No microphone here.'}
      {#if micState !== 'unavailable'}
        <button class="font-semibold text-primary hover:underline" onclick={() => void startMic()}>Try again</button>
      {/if}
    </div>
  {/if}

  {#if hud.countdown > 0}
    <div class="pointer-events-none absolute inset-0 flex items-center justify-center text-8xl font-black text-white/90">
      {hud.countdown}
    </div>
  {/if}

  <div class="pointer-events-none absolute top-4 right-5 text-right">
    <div class="font-mono text-3xl font-bold tabular-nums">{hud.score.toLocaleString()}</div>
    <div class="mt-1 flex items-center justify-end gap-2">
      <span
        class="rounded-md px-2 py-0.5 font-mono text-lg font-black {hud.starActive
          ? 'bg-cyan-300 text-black'
          : hud.multiplier >= 4
            ? 'bg-primary text-primary-foreground'
            : 'bg-white/10 text-white'}">{hud.multiplier}x</span
      >
      <span class="font-mono text-sm transition-colors {hud.missing ? 'text-red-400' : 'text-neutral-300'}">{hud.streak} streak</span>
    </div>
    <div class="mt-2 ml-auto h-2 w-40 overflow-hidden rounded-full bg-white/10">
      <div class="h-full bg-cyan-300 transition-[width] duration-100" style="width: {Math.round(hud.star * 100)}%"></div>
    </div>
    <div class="mt-1 text-[11px] text-neutral-400">
      {hud.star >= 0.5 && !hud.starActive ? 'Star power ready' : hud.starActive ? 'Star power!' : 'Star power'}
    </div>
  </div>

  <div class="absolute top-4 left-5">
    <div class="pointer-events-none">
      <div class="text-sm font-semibold">{song.name}</div>
      <div class="text-xs text-neutral-400">{song.artist}</div>
      {#if hud.section}<div class="mt-1 text-xs tracking-wider text-primary uppercase">{hud.section}</div>{/if}
    </div>
    {#if room.host === conn.conn}
      <button
        class="mt-3 flex items-center gap-1.5 rounded-lg border border-white/10 bg-black/60 px-3 py-1.5 text-xs text-neutral-300 hover:border-white/30 hover:text-white"
        onclick={() => conn.send({ type: 'stop' })}
      >
        <X class="size-3.5" /> Exit song
      </button>
    {/if}
  </div>

  <div class="absolute bottom-4 left-4 w-64">
    <BandList {conn} live compact />
  </div>

  {#if hud.done}
    <div class="pointer-events-none absolute bottom-6 left-1/2 -translate-x-1/2 rounded-lg bg-black/70 px-4 py-2 text-sm text-neutral-300">
      Waiting for the rest of the band to finish
    </div>
  {/if}
</div>
