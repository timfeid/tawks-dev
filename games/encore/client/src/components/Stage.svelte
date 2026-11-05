<script lang="ts">
  import { onDestroy, onMount, untrack } from 'svelte';
  import { X } from '@lucide/svelte';
  import { SongPlayer, playScratch } from '../lib/audio';
  import { DrumKit } from '../lib/drums';
  import { CUE_HOLD_MS, StemGate } from '../lib/gate';
  import { Engine, type Action } from '../lib/engine';
  import { Controls, type Control, type Layout } from '../lib/input';
  import { drawHighway } from '../lib/render';
  import { drawVocals, pitchRange } from '../lib/render-vocals';
  import { VocalEngine, hzToMidi, type VoiceSample } from '../lib/vocals';
  import { host, listen, type HeardSample } from '../lib/voice';
  import type { MicrophoneState } from '@tawks/activity-sdk';
  import type { RoomConnection } from '../lib/room.svelte';
  import { readSettings, type Settings } from '../lib/settings';
  import { autoStep, playSuggestion } from '../lib/calibration';
  import { DIFFICULTIES, INSTRUMENT_LABELS, type Chart, type Instrument, type PartChoice, type Track } from '../lib/types';
  import BandList from './BandList.svelte';
  import Lyrics from './Lyrics.svelte';

  let { conn, settings }: { conn: RoomConnection; settings: Settings } = $props();

  const PROGRESS_EVERY_MS = 200;
  const PRE_SONG_MS = 1e9;
  const params = new URLSearchParams(location.search);
  /**
   * For testing: `?autoplay` plays every note perfectly. `?autoplay=sloppy`
   * skips every other four seconds of notes. On vocals, `?autoplay=offkey`
   * sings a tritone off and `?autoplay=alternate` swaps between the two every
   * four seconds. `?nocues` ignores the band's cues, leaving stems to follow
   * progress messages alone.
   */
  const autoplay = params.has('autoplay');
  const autoMode = params.get('autoplay') ?? '';
  let autoIndex = 0;
  const AUTO_KICK_AHEAD_MS = 120;
  const MILESTONE_EVERY = 50;

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
  let hud = $state({
    score: 0,
    multiplier: 1,
    streak: 0,
    star: 0,
    starActive: false,
    countdown: 0,
    section: '',
    done: false,
    missing: false,
    autoKick: false,
  });
  /** Bumped to replay the multiplier pop, and the callout under the score. */
  let multiplierPops = $state(0);
  let callout = $state<{ id: number; text: string; bad: boolean } | null>(null);
  let calloutTimer: ReturnType<typeof setTimeout> | null = null;

  let engine: Engine | null = null;
  let vocal: VocalEngine | null = null;
  let range = { low: 55, high: 72 };
  let voice: { state: MicrophoneState; stop: () => void } | null = null;
  let micState = $state<MicrophoneState | null>(null);
  const heardQueue: HeardSample[] = [];
  let player: SongPlayer | null = null;
  let kit: DrumKit | null = null;
  let gate: StemGate | null = null;
  let stopCues: (() => void) | null = null;
  let lastCueAt = -1e9;
  let kickSoundIndex = 0;
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
        engine = new Engine(track, chart.beats, { autoKick: layout === 'drums' && settings.autoKick });
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
      gate = new StemGate((instrument, out, at) => player?.setMissing(instrument, out, at));
      if (!params.has('nocues')) stopCues = conn.onCue((from, t, out) => gate?.cue(from, t, out, CUE_HOLD_MS));
      if (singing && !settings.vocalGuide) player.silence('vocals');
      if (autoplay || params.has('debug')) {
        const p = player;
        (window as unknown as { encore: unknown }).encore = {
          gain: (part: Instrument) => p.gainOf(part),
          songTime: () => p.songTime(),
          engine: () => engine ?? vocal,
        };
      }
      status = 'ready';
      if (singing) void startMic();
      else {
        if (layout === 'drums') kit = new DrumKit(player.ctx, settings.drumVolume);
        controls = new Controls(settings.bindings[layout], (control, down, at) => {
          if (kit && down && control.startsWith('lane') && !(control === 'lane0' && engine?.autoKick)) kit.play(Number(control.slice(4)));
          queue.push({ control, down, at });
        });
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
    stopCues?.();
    if (lobbyMuted) host.activity?.singing(false);
    kit?.dispose();
    player?.dispose();
    if (calloutTimer) clearTimeout(calloutTimer);
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
    return { score: 0, streak: 0, best_streak: 0, multiplier: 1, hit: 0, missed: 0, total: 0, star_meter: 0, star_active: false, muted: false, auto_kick: false };
  }

  /**
   * Keeps every part's stem following its players. This player's own drops
   * apply on the spot and go to the band as cues stamped with song time.
   */
  function applyMissing(heard: number) {
    const scorer = engine ?? vocal;
    if (!player || !scorer || !gate || conn.conn === null) return;
    const players = room.players
      .filter((p) => p.playing && p.part)
      .map((p) => ({ conn: p.conn, instrument: p.part!.instrument, out: p.conn === conn.conn ? scorer.muted : p.progress.muted }));
    gate.roster(players, heard);
    if (scorer.mutedAt !== lastCueAt) {
      lastCueAt = scorer.mutedAt;
      gate.cue(conn.conn, scorer.mutedAt, scorer.muted, 0);
      conn.send({ type: 'cue', t: scorer.mutedAt, out: scorer.muted });
    }
  }

  /** After a full song, notes how the hits landed and, with auto adjust on, moves the audio offset a step. */
  function learnTiming(engine: Engine) {
    const late = playSuggestion(engine.hitErrors);
    conn.timing = null;
    if (late === null) return;
    if (!settings.autoAdjust) {
      conn.timing = { late, adjusted: null };
      return;
    }
    const saved = readSettings(conn.settings);
    const inputOffset = autoStep(saved.inputOffset, late);
    conn.saveSettings({ ...saved, inputOffset });
    conn.timing = { late, adjusted: inputOffset - saved.inputOffset };
  }

  /** Schedules the kick sound for each auto kick just ahead, on the song's own audio clock. */
  function scheduleAutoKicks(engine: Engine, heard: number) {
    if (!kit || !player || !engine.autoKick) return;
    const notes = engine.notes;
    while (kickSoundIndex < notes.length && notes[kickSoundIndex].t < heard) kickSoundIndex++;
    for (; kickSoundIndex < notes.length && notes[kickSoundIndex].t <= heard + AUTO_KICK_AHEAD_MS; kickSoundIndex++) {
      if (engine.isAutoKick(kickSoundIndex)) kit.play(0, player.contextTime(notes[kickSoundIndex].t));
    }
  }

  function playAhead(engine: Engine, until: number) {
    for (; autoIndex < engine.notes.length && engine.notes[autoIndex].t <= until; autoIndex++) {
      const note = engine.notes[autoIndex];
      if (autoMode === 'sloppy' && Math.floor(note.t / 4000) % 2 === 1) continue;
      if (engine.drums) {
        if (engine.isAutoKick(autoIndex)) continue;
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
      const offKey = autoMode === 'offkey' || (autoMode === 'alternate' && Math.floor(t / 4000) % 2 === 1);
      samples.push({ time: t, midi: i >= 0 ? vocal.notes[i].pitch + (offKey ? 6 : 0) : null, level: i >= 0 ? 0.1 : 0 });
    }
    vocal.feed(samples);
  }

  let lastSung = 0;
  let lobbyMuted = false;

  /** Holds the lobby mic muted from the count-in to the end of the song. */
  function holdLobbyMic(want: boolean) {
    if (want === lobbyMuted) return;
    lobbyMuted = want;
    host.activity?.singing(want);
  }

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
      applyMissing(heard);
      const done = heard > songEnd;
      holdLobbyMic(settings.muteLobbyWhileSinging && !done);
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

  function say(text: string, bad = false) {
    callout = { id: (callout?.id ?? 0) + 1, text, bad };
    if (calloutTimer) clearTimeout(calloutTimer);
    calloutTimer = setTimeout(() => (callout = null), 1200);
  }

  function updateHud(scorer: Engine | VocalEngine, heard: number) {
    lyricTime = heard;
    const section = chart!.sections.findLast((s) => s.t <= heard)?.name ?? '';
    if (scorer.multiplier > hud.multiplier) {
      multiplierPops++;
      if (!scorer.starActive || hud.starActive) say(`${scorer.multiplier}x multiplier`);
    }
    if (scorer.streak > 0 && scorer.streak % MILESTONE_EVERY === 0 && scorer.streak !== hud.streak) say(`${scorer.streak} note streak`);
    if (hud.streak >= 10 && scorer.streak === 0) say('Streak lost', true);
    hud = {
      autoKick: engine?.autoKick ?? false,
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
      scheduleAutoKicks(engine, heard);
      if (engine.overstrumAt !== lastOverstrum) {
        lastOverstrum = engine.overstrumAt;
        if (judged > 0) playScratch(settings.volume * 0.6);
      }
      applyMissing(judged);
      const done = heard > songEnd;
      if (done && !sentDone) learnTiming(engine);
      if ((now - lastSent > PROGRESS_EVERY_MS && !sentDone) || (done && !sentDone)) {
        lastSent = now;
        sentDone = done;
        conn.send({ type: 'progress', progress: engine.progress(), done });
      }
    } else {
      // Before the song starts, presses still light their targets on a stand-in clock far before the first note.
      heard = now - PRE_SONG_MS;
      for (const event of queue.splice(0)) {
        const action: Action = event.control === 'strum_up' || event.control === 'strum_down' ? 'strum' : event.control;
        if (action !== 'strum') engine.input({ action, down: event.down, time: event.at - PRE_SONG_MS - settings.inputOffset });
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
      {#key multiplierPops}
        <span
          class="pop rounded-md px-2 py-0.5 font-mono text-lg font-black {hud.starActive
            ? 'bg-cyan-300 text-black'
            : hud.multiplier >= 4
              ? 'bg-primary text-primary-foreground'
              : 'bg-white/10 text-white'}">{hud.multiplier}x</span
        >
      {/key}
      <span class="font-mono text-sm transition-colors {hud.missing ? 'text-red-400' : 'text-neutral-300'}">{hud.streak} streak</span>
    </div>
    {#if !singing}
      <div class="mt-1.5 ml-auto flex w-40 gap-0.5" aria-hidden="true">
        {#each Array.from({ length: 10 }, (_, i) => i) as pip (pip)}
          <span
            class="h-1 flex-1 rounded-full {hud.multiplier >= 4 || hud.streak % 10 > pip
              ? hud.missing
                ? 'bg-red-400'
                : 'bg-primary'
              : 'bg-white/10'}"
          ></span>
        {/each}
      </div>
    {/if}
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
      {#if hud.autoKick}<div class="mt-1 text-xs text-orange-300">Auto kick on, no best saved</div>{/if}
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

  {#if callout}
    {#key callout.id}
      <div
        class="callout pointer-events-none absolute top-[38%] left-1/2 -translate-x-1/2 font-black tracking-wide whitespace-nowrap {callout.bad
          ? 'text-2xl text-red-400'
          : 'text-3xl text-white'}"
      >
        {callout.text}
      </div>
    {/key}
  {/if}

  {#if hud.done}
    <div class="pointer-events-none absolute bottom-6 left-1/2 -translate-x-1/2 rounded-lg bg-black/70 px-4 py-2 text-sm text-neutral-300">
      Waiting for the rest of the band to finish
    </div>
  {/if}
</div>

<style>
  .pop {
    animation: pop 260ms ease-out;
  }
  .callout {
    animation: rise 1200ms ease-out forwards;
    text-shadow: 0 2px 12px rgba(0, 0, 0, 0.8);
  }
  @keyframes pop {
    0% {
      transform: scale(1.6);
    }
    100% {
      transform: scale(1);
    }
  }
  @keyframes rise {
    0% {
      opacity: 0;
      transform: translate(-50%, 8px) scale(0.9);
    }
    15% {
      opacity: 1;
      transform: translate(-50%, 0) scale(1);
    }
    75% {
      opacity: 1;
    }
    100% {
      opacity: 0;
      transform: translate(-50%, -12px);
    }
  }
</style>
