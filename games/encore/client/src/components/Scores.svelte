<script lang="ts">
  import { LoaderCircle } from '@lucide/svelte';
  import type { RoomConnection } from '../lib/room.svelte';
  import { fetchChartScores } from '../lib/songs';
  import { INSTRUMENT_LABELS, type ChartScores, type PartChoice, type ScoreEntry, type SongInfo } from '../lib/types';
  import FullCombo from './FullCombo.svelte';
  import Stars from './Stars.svelte';

  let { conn, song, part }: { conn: RoomConnection; song: SongInfo; part: PartChoice } = $props();

  let everyone = $state(false);
  let scores = $state<ChartScores | null>(null);
  let failed = $state(false);
  let loading = $state(false);

  const official = $derived(!!song.official);
  const scope = $derived(everyone && official);
  const songId = $derived(song.id);
  const instrument = $derived(part.instrument);
  const difficulty = $derived(part.difficulty);
  const mineShown = $derived(!!scores?.mine && scores.entries.some((e) => e.user_id === conn.userId));

  // Room updates hand over a new `part` object each time, so only the chart's own values refetch.
  $effect(() => {
    void conn.scoresVersion;
    const ask = { songId, part: { instrument, difficulty }, everyone: scope };
    let stale = false;
    loading = true;
    const timer = setTimeout(async () => {
      try {
        const result = await fetchChartScores(conn, ask.songId, ask.part, ask.everyone);
        if (stale) return;
        scores = result;
        failed = false;
      } catch {
        if (!stale) failed = true;
      } finally {
        if (!stale) loading = false;
      }
    }, 150);
    return () => {
      stale = true;
      clearTimeout(timer);
    };
  });
</script>

{#snippet row(entry: ScoreEntry)}
  <div
    class="flex items-center gap-2 rounded-md px-2 py-1 text-sm {entry.user_id === conn.userId ? 'bg-primary/10' : ''}"
  >
    <span class="w-6 shrink-0 text-right font-mono text-xs text-neutral-500">{entry.rank}</span>
    <span class="min-w-0 flex-1 truncate">{entry.name}{entry.user_id === conn.userId ? ' (you)' : ''}</span>
    {#if entry.full_combo}<FullCombo />{/if}
    <Stars stars={entry.stars} />
    <span class="w-20 text-right font-mono text-xs">{entry.score.toLocaleString()}</span>
  </div>
{/snippet}

<div class="space-y-2" data-testid="encore-scores">
  <div class="flex items-center gap-2">
    <div class="text-xs font-semibold tracking-wider text-neutral-500 uppercase">Scores</div>
    {#if loading}<LoaderCircle class="size-3 animate-spin text-neutral-500" />{/if}
    <div class="ml-auto flex rounded-lg border border-white/10 p-0.5 text-xs">
      <button
        class="rounded-md px-2.5 py-0.5 transition-colors {!scope ? 'bg-white/10 text-white' : 'text-neutral-400 hover:text-white'}"
        onclick={() => (everyone = false)}
      >
        Friends
      </button>
      <button
        class="rounded-md px-2.5 py-0.5 transition-colors disabled:opacity-40 {scope ? 'bg-white/10 text-white' : 'text-neutral-400 hover:text-white'}"
        disabled={!official}
        title={official ? 'Everyone who played this chart' : 'Only official songs rank everyone'}
        onclick={() => (everyone = true)}
      >
        Everyone
      </button>
    </div>
  </div>
  <div class="text-xs text-neutral-500">
    {INSTRUMENT_LABELS[part.instrument]}, <span class="capitalize">{part.difficulty}</span>
    {#if scores?.mine}
      <span class="text-neutral-300"> / you are #{scores.mine.rank} of {scores.players}</span>
    {/if}
  </div>
  {#if failed && !scores}
    <p class="text-xs text-neutral-500">Scores did not load.</p>
  {:else if scores && !scores.entries.length}
    <p class="text-xs leading-relaxed text-neutral-500">
      {scope ? 'Nobody has played this part through yet.' : 'No scores from you or your friends on this part yet.'}
    </p>
  {:else if scores}
    <div class="max-h-56 space-y-0.5 overflow-y-auto">
      {#each scores.entries as entry (entry.user_id)}
        {@render row(entry)}
      {/each}
      {#if scores.mine && !mineShown}
        <div class="mx-2 border-t border-white/10"></div>
        {@render row(scores.mine)}
      {/if}
    </div>
  {/if}
</div>
