<script lang="ts">
  import { Trophy, X } from '@lucide/svelte';
  import type { RoomConnection } from '../lib/room.svelte';
  import { INSTRUMENT_LABELS, type SongInfo } from '../lib/types';
  import FullCombo from './FullCombo.svelte';
  import Stars from './Stars.svelte';

  let { conn, catalog, onClose }: { conn: RoomConnection; catalog: SongInfo[]; onClose: () => void } = $props();

  const SHOWN = 25;
  const songs = $derived(new Map(catalog.map((song) => [song.id, song])));
  const top = $derived(
    Object.values(conn.bests)
      .filter((best) => songs.has(best.song_id))
      .sort((a, b) => b.score - a.score)
      .slice(0, SHOWN),
  );
</script>

<div class="absolute inset-0 z-20 flex items-center justify-center bg-black/70 p-4">
  <div class="flex max-h-full w-full max-w-lg flex-col overflow-hidden rounded-2xl border border-white/10 bg-background shadow-2xl">
    <header class="flex items-center gap-2 border-b border-white/10 px-5 py-3">
      <Trophy class="size-4 text-amber-300" />
      <h2 class="font-semibold">My best scores</h2>
      <button class="ml-auto rounded-md p-1 text-neutral-400 hover:bg-white/10 hover:text-white" onclick={onClose} aria-label="Close">
        <X class="size-4" />
      </button>
    </header>
    <div class="min-h-0 flex-1 overflow-y-auto p-3">
      {#if !top.length}
        <p class="p-3 text-sm text-neutral-400">Play a song through and your best runs show up here.</p>
      {/if}
      {#each top as best (`${best.song_id}:${best.instrument}:${best.difficulty}`)}
        {@const song = songs.get(best.song_id)}
        <div class="flex items-center gap-3 rounded-lg px-2 py-2 hover:bg-white/5">
          <div class="min-w-0 flex-1">
            <div class="truncate text-sm font-semibold">{song?.name}</div>
            <div class="truncate text-xs text-neutral-400">
              {song?.artist} / {INSTRUMENT_LABELS[best.instrument]}, <span class="capitalize">{best.difficulty}</span>
            </div>
          </div>
          {#if best.full_combo}<FullCombo />{/if}
          <Stars stars={best.stars} />
          <span class="w-24 text-right font-mono text-sm">{best.score.toLocaleString()}</span>
        </div>
      {/each}
    </div>
  </div>
</div>
