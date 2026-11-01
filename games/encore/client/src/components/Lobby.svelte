<script lang="ts">
  import { BadgeCheck, Check, Drum, Flag, Gamepad2, Guitar, Mic, Music, Piano, Search, Trash2, Upload as UploadIcon, X } from '@lucide/svelte';
  import { unlockAudio } from '../lib/audio';
  import { deleteSong } from '../lib/songs';
  import type { RoomConnection } from '../lib/room.svelte';
  import type { Settings } from '../lib/settings';
  import {
    DIFFICULTIES,
    INSTRUMENT_LABELS,
    bestKey,
    formatLength,
    type Difficulty,
    type Instrument,
    type SongInfo,
  } from '../lib/types';
  import BandList from './BandList.svelte';
  import Report from './Report.svelte';
  import Upload from './Upload.svelte';

  let {
    conn,
    catalog,
    catalogFailed,
    catalogLoaded,
    settings,
    onSetup,
    onCatalogChange,
  }: {
    conn: RoomConnection;
    catalog: SongInfo[];
    catalogFailed: boolean;
    catalogLoaded: boolean;
    settings: Settings;
    onSetup: () => void;
    onCatalogChange: () => void;
  } = $props();

  let uploading = $state(false);
  let reporting = $state<SongInfo | null>(null);
  let confirmDelete = $state<string | null>(null);

  const mine = (entry: SongInfo) => !!conn.userId && entry.uploaded_by === conn.userId;
  const reportable = (entry: SongInfo) => entry.source === 'upload' && !mine(entry);

  function uploader(entry: SongInfo) {
    if (entry.official) return 'Official';
    if (entry.source !== 'upload') return null;
    return mine(entry) ? 'Your upload' : `Uploaded by ${entry.uploader_name ?? 'a player'}`;
  }

  async function remove(entry: SongInfo) {
    if (confirmDelete !== entry.id) {
      confirmDelete = entry.id;
      return;
    }
    confirmDelete = null;
    try {
      await deleteSong(conn, entry.id);
      onCatalogChange();
    } catch (error) {
      conn.flash((error as Error).message);
    }
  }

  const SHOWN = 200;
  const ICONS = { guitar: Guitar, bass: Guitar, rhythm: Guitar, keys: Piano, drums: Drum, vocals: Mic };

  let query = $state('');
  let brokenArt = $state<string | null>(null);
  let partFilter = $state<Instrument | 'any'>('any');

  const room = $derived(conn.room!);
  const song = $derived(room.song);
  const me = $derived(conn.me);
  const busy = $derived(room.phase === 'loading' || room.phase === 'playing');

  const matches = $derived.by(() => {
    const words = query.toLowerCase().split(/\s+/).filter(Boolean);
    return catalog.filter((s) => {
      if (partFilter !== 'any' && !s.parts.some((p) => p.instrument === partFilter)) return false;
      const text = `${s.name} ${s.artist} ${s.album ?? ''} ${s.genre ?? ''}`.toLowerCase();
      return words.every((w) => text.includes(w));
    });
  });

  function art(id: string) {
    const pass = conn.current;
    return `${pass.api_url}/songs/${id}/files/album?token=${encodeURIComponent(pass.token)}`;
  }

  function pick(id: string) {
    unlockAudio();
    conn.send({ type: 'pick', song_id: id });
  }

  function choose(instrument: Instrument | null, difficulty: Difficulty | null = me?.part?.difficulty ?? 'medium') {
    unlockAudio();
    conn.send({ type: 'part', instrument, difficulty: instrument ? difficulty : null });
  }

  function toggleReady() {
    unlockAudio();
    conn.send({ type: 'ready', ready: !me?.ready });
  }

  const myBest = $derived(song && me?.part ? conn.bests[bestKey(song.id, me.part)] : undefined);
  const layoutHint = $derived(me?.part?.instrument === 'drums' ? 'drums' : 'guitar');
  const firstBinding = $derived(settings.bindings[layoutHint].lane0[0]);
</script>

<div class="grid h-full grid-cols-1 md:grid-cols-[minmax(0,1fr)_380px]">
  <section class="flex min-h-0 flex-col border-r border-white/10">
    <header class="space-y-3 border-b border-white/10 px-5 py-4">
      <div class="flex items-center gap-2">
        <Music class="size-5 shrink-0 text-primary" />
        <h1 class="text-lg font-bold tracking-tight">Encore</h1>
        <span class="text-sm whitespace-nowrap text-neutral-500">{catalog.length.toLocaleString()} songs</span>
        {#if conn.uploads}
          <button
            class="ml-auto flex items-center gap-1.5 rounded-lg border border-white/10 px-3 py-1 text-sm hover:border-white/30"
            onclick={() => (uploading = true)}
          >
            <UploadIcon class="size-4" /> Upload a song
          </button>
        {/if}
      </div>
      <div class="flex items-center gap-2 rounded-lg bg-white/5 px-3 py-1.5">
        <Search class="size-4 shrink-0 text-neutral-500" />
        <input
          bind:value={query}
          placeholder="Search songs, artists, albums"
          class="min-w-0 flex-1 bg-transparent text-sm outline-none placeholder:text-neutral-500"
        />
      </div>
      <div class="flex flex-wrap gap-1.5">
        {#each ['any', 'guitar', 'bass', 'drums', 'keys', 'vocals'] as const as filter (filter)}
          <button
            class="rounded-full border px-3 py-0.5 text-xs transition-colors {partFilter === filter
              ? 'border-primary/60 bg-primary/15 text-primary'
              : 'border-white/10 text-neutral-400 hover:text-white'}"
            onclick={() => (partFilter = filter)}
          >
            {filter === 'any' ? 'Any part' : INSTRUMENT_LABELS[filter]}
          </button>
        {/each}
      </div>
    </header>
    <div class="min-h-0 flex-1 overflow-y-auto">
      {#if catalogFailed}
        <p class="p-6 text-sm text-neutral-400">The song list did not load.</p>
      {:else if !catalogLoaded}
        <p class="p-6 text-sm text-neutral-400">Loading the song list...</p>
      {:else if !catalog.length}
        <p class="p-6 text-sm leading-relaxed text-neutral-400">
          {conn.uploads
            ? 'No songs here yet. Upload a song you own or have the right to use, and the people in this room can play it with you.'
            : 'No songs here yet.'}
        </p>
      {/if}
      {#each matches.slice(0, SHOWN) as entry (entry.id)}
        <div class="flex items-center transition-colors hover:bg-white/5 {song?.id === entry.id ? 'bg-primary/10' : ''}">
        <button
          class="flex min-w-0 flex-1 items-center gap-3 py-2 pl-5 text-left disabled:opacity-50 {reportable(entry) ? 'pr-2' : 'pr-5'}"
          disabled={busy}
          onclick={() => pick(entry.id)}
        >
          <div class="min-w-0 flex-1">
            <div class="truncate text-sm font-semibold">{entry.name}</div>
            <div class="truncate text-xs text-neutral-400">
              {entry.artist}{entry.year ? ` (${entry.year})` : ''}{#if uploader(entry)}<span class="text-neutral-500"> / {uploader(entry)}</span>{/if}
            </div>
          </div>
          <div class="flex gap-1 text-neutral-500">
            {#each entry.parts as part (part.instrument)}
              {@const Icon = ICONS[part.instrument]}
              <span title={INSTRUMENT_LABELS[part.instrument]}><Icon class="size-3.5" /></span>
            {/each}
          </div>
          <span class="w-10 text-right font-mono text-xs text-neutral-500">{formatLength(entry.length_ms)}</span>
        </button>
        {#if reportable(entry)}
          <button
            class="mr-3 rounded-md p-1.5 text-neutral-500 hover:bg-white/10 hover:text-white"
            title="Report this song"
            aria-label="Report this song"
            onclick={() => (reporting = entry)}
          >
            <Flag class="size-3.5" />
          </button>
        {/if}
        </div>
      {/each}
      {#if matches.length > SHOWN}
        <p class="px-5 py-3 text-xs text-neutral-500">Showing {SHOWN} of {matches.length}. Search to narrow it down.</p>
      {/if}
    </div>
  </section>

  <aside class="flex min-h-0 flex-col gap-4 overflow-y-auto p-5">
    {#if song}
      <div class="flex gap-4">
        {#if song.art && brokenArt !== song.id}
          <img
            src={art(song.id)}
            alt=""
            class="size-24 shrink-0 rounded-lg object-cover"
            onerror={() => (brokenArt = song.id)}
          />
        {:else}
          <div
            class="flex size-24 shrink-0 items-center justify-center rounded-lg border border-primary/20 bg-gradient-to-br from-primary/20 to-black"
          >
            <Music class="size-9 text-primary/80" />
          </div>
        {/if}
        <div class="min-w-0">
          <div class="text-xs tracking-wider text-neutral-500 uppercase">
            {busy ? 'Now playing' : `Picked by ${room.picked_by ?? 'someone'}`}
          </div>
          <div class="text-lg leading-tight font-bold">{song.name}</div>
          <div class="text-sm text-neutral-400">{song.artist}</div>
          <div class="mt-1 text-xs text-neutral-500">
            {formatLength(song.length_ms)}{song.charter ? ` / charted by ${song.charter}` : ''}
          </div>
          {#if uploader(song)}
            <div class="mt-1 flex items-center gap-1 text-xs text-neutral-500">
              {#if song.official}<BadgeCheck class="size-3.5 text-primary" />{/if}
              {uploader(song)}
            </div>
          {/if}
        </div>
      </div>

      {#if song.source === 'upload' && !busy}
        <div class="flex gap-1.5">
          {#if mine(song)}
            <button
              class="flex items-center gap-1.5 rounded-lg border border-white/10 px-3 py-1 text-xs text-neutral-300 hover:border-white/30"
              onclick={() => remove(song)}
            >
              <Trash2 class="size-3.5" />
              {confirmDelete === song.id ? 'Delete it for good' : 'Delete this song'}
            </button>
          {:else}
            <button
              class="flex items-center gap-1.5 rounded-lg border border-white/10 px-3 py-1 text-xs text-neutral-300 hover:border-white/30"
              onclick={() => (reporting = song)}
            >
              <Flag class="size-3.5" /> Report
            </button>
          {/if}
        </div>
      {/if}

      {#if !busy}
        <div class="space-y-2">
          <div class="text-xs font-semibold tracking-wider text-neutral-500 uppercase">Your part</div>
          <div class="flex flex-wrap gap-1.5">
            {#each song.parts as part (part.instrument)}
              {@const Icon = ICONS[part.instrument]}
              <button
                class="flex items-center gap-1.5 rounded-lg border px-3 py-1.5 text-sm transition-colors {me?.part?.instrument ===
                part.instrument
                  ? 'border-primary bg-primary/15'
                  : 'border-white/10 hover:border-white/30'}"
                onclick={() => choose(part.instrument)}
              >
                <Icon class="size-4" />
                {INSTRUMENT_LABELS[part.instrument]}
              </button>
            {/each}
            <button
              class="rounded-lg border px-3 py-1.5 text-sm transition-colors {!me?.part
                ? 'border-white/40 bg-white/10'
                : 'border-white/10 text-neutral-400 hover:border-white/30'}"
              onclick={() => choose(null)}
            >
              Watch
            </button>
          </div>
          {#if me?.part}
            <div class="flex gap-1.5">
              {#each DIFFICULTIES as difficulty (difficulty)}
                <button
                  class="flex-1 rounded-lg border px-2 py-1 text-xs capitalize transition-colors {me.part.difficulty === difficulty
                    ? 'border-primary bg-primary/15'
                    : 'border-white/10 text-neutral-400 hover:border-white/30'}"
                  onclick={() => choose(me.part!.instrument, difficulty)}
                >
                  {difficulty}
                </button>
              {/each}
            </div>
            {#if myBest}
              <div class="text-xs text-neutral-400">Your best on this part: <span class="font-mono text-white">{myBest.toLocaleString()}</span></div>
            {/if}
            <button
              class="flex w-full items-center justify-center gap-2 rounded-lg py-2.5 text-sm font-bold transition-colors {me.ready
                ? 'bg-secondary text-primary hover:bg-secondary/80'
                : 'bg-primary text-primary-foreground hover:bg-primary/90'}"
              onclick={toggleReady}
            >
              {#if me.ready}<Check class="size-4" /> Ready{:else}I'm ready{/if}
            </button>
          {/if}
        </div>
      {/if}
    {:else}
      <div class="rounded-lg border border-dashed border-white/15 p-6 text-center text-sm text-neutral-400">
        Pick a song from the list. Anyone in the lobby can choose.
      </div>
    {/if}

    {#if busy && room.host === conn.conn}
      <button
        class="flex w-full items-center justify-center gap-2 rounded-lg border border-white/10 py-2 text-sm hover:border-white/30"
        onclick={() => conn.send({ type: 'stop' })}
      >
        <X class="size-4" /> Exit song
      </button>
    {/if}

    <BandList {conn} live={busy} />

    <div class="mt-auto space-y-2 border-t border-white/10 pt-4">
      <button
        class="flex w-full items-center justify-center gap-2 rounded-lg border border-white/10 py-2 text-sm hover:border-white/30"
        onclick={onSetup}
      >
        <Gamepad2 class="size-4" /> Controller and calibration
      </button>
      <p class="text-xs leading-relaxed text-neutral-500">
        Plug in a guitar or drum kit, or play on the keyboard{firstBinding?.kind === 'key' ? ` (${layoutHint === 'drums' ? 'Space, J, K, L, ;' : 'A S D F G to fret, arrows or Enter to strum'})` : ''}.
        Headphones help: your mic would pick up the song.
        {#if conn.rtt !== null}<span class="font-mono">Ping {conn.rtt} ms.</span>{/if}
      </p>
    </div>
  </aside>
</div>

{#if uploading}
  <Upload {conn} onClose={() => (uploading = false)} onUploaded={onCatalogChange} />
{/if}
{#if reporting}
  <Report {conn} song={reporting} onClose={() => (reporting = null)} />
{/if}
