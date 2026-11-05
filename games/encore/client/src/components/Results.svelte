<script lang="ts">
  import { Trophy } from '@lucide/svelte';
  import type { RoomConnection } from '../lib/room.svelte';
  import { INSTRUMENT_LABELS, bestKey } from '../lib/types';

  let { conn }: { conn: RoomConnection } = $props();

  const room = $derived(conn.room!);
  const band = $derived(
    room.players.filter((p) => p.playing).sort((a, b) => b.progress.score - a.progress.score),
  );
  const total = $derived(band.reduce((sum, p) => sum + p.progress.score, 0));
  const accuracy = (hit: number, all: number) => (all ? Math.round((hit / all) * 100) : 0);

  // The server keeps the new best; remember the old one here to tell the player.
  const mine = conn.me;
  const key = mine?.part && conn.room?.song ? bestKey(conn.room.song.id, mine.part) : null;
  const previousBest = key ? (conn.bests[key] ?? 0) : 0;
  const newBest = !!key && !!mine && !mine.progress.auto_kick && mine.progress.score > previousBest;
  if (key && mine && newBest) conn.bests = { ...conn.bests, [key]: mine.progress.score };
</script>

<div class="flex h-full flex-col items-center justify-center gap-6 p-6">
  <div class="text-center">
    <div class="text-xs tracking-widest text-neutral-500 uppercase">Song complete</div>
    <h2 class="text-2xl font-bold">{room.song?.name}</h2>
    <div class="text-sm text-neutral-400">{room.song?.artist}</div>
    <div class="mt-2 font-mono text-3xl font-bold text-primary">{total.toLocaleString()}</div>
    <div class="text-xs text-neutral-500">band score</div>
  </div>

  <div class="w-full max-w-xl space-y-2">
    {#each band as player, i (player.conn)}
      <div class="flex items-center gap-3 rounded-xl bg-white/[0.05] px-4 py-3">
        {#if i === 0}<Trophy class="size-5 text-amber-300" />{:else}<span class="w-5 text-center font-mono text-neutral-500">{i + 1}</span>{/if}
        <div class="min-w-0 flex-1">
          <div class="truncate font-semibold">{player.name}</div>
          <div class="text-xs text-neutral-400">
            {player.part ? INSTRUMENT_LABELS[player.part.instrument] : ''}, <span class="capitalize">{player.part?.difficulty}</span>
            {#if player.progress.auto_kick}<span class="ml-1 rounded bg-orange-400/15 px-1.5 py-0.5 text-[11px] text-orange-300">Auto kick</span>{/if}
          </div>
        </div>
        <div class="text-right font-mono text-xs text-neutral-400">
          <div>{accuracy(player.progress.hit, player.progress.total)}% hit</div>
          <div>best streak {player.progress.best_streak}</div>
        </div>
        <div class="w-28 text-right">
          <div class="font-mono text-lg font-bold">{player.progress.score.toLocaleString()}</div>
          {#if player.conn === conn.conn && newBest}
            <div class="text-[11px] font-semibold text-primary">New best</div>
          {/if}
        </div>
      </div>
    {/each}
  </div>

  <button
    class="rounded-lg bg-primary px-6 py-2.5 text-sm font-bold text-primary-foreground hover:bg-primary/90"
    onclick={() => conn.send({ type: 'again' })}
  >
    Pick the next song
  </button>
</div>
