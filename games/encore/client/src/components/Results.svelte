<script lang="ts">
  import { Trophy } from '@lucide/svelte';
  import type { RoomConnection } from '../lib/room.svelte';
  import { INSTRUMENT_LABELS, type Standing } from '../lib/types';
  import { readSettings } from '../lib/settings';
  import FullCombo from './FullCombo.svelte';
  import Stars from './Stars.svelte';

  let { conn }: { conn: RoomConnection } = $props();

  const room = $derived(conn.room!);
  const band = $derived(
    room.players.filter((p) => p.playing).sort((a, b) => b.progress.score - a.progress.score),
  );
  const total = $derived(band.reduce((sum, p) => sum + p.progress.score, 0));
  const accuracy = (hit: number, all: number) => (all ? Math.round((hit / all) * 100) : 0);

  const kept = $derived(conn.lastRun);
  const newBest = $derived(!!kept?.new_best);
  const describe = (standing: Standing) => (standing.rank ? `#${standing.rank} of ${standing.players}` : 'not ranked');

  const timing = conn.timing;
  conn.timing = null;
  let timingOpen = $state(!!timing);
  const describeLate = (ms: number) => `${Math.abs(ms)} ms ${ms > 0 ? 'late' : 'early'}`;

  function applyTiming() {
    if (!timing) return;
    const saved = readSettings(conn.settings);
    conn.saveSettings({ ...saved, inputOffset: Math.max(-300, Math.min(300, saved.inputOffset + timing.late)) });
    timingOpen = false;
  }
</script>

<div class="flex h-full flex-col items-center justify-center gap-6 p-6">
  <div class="text-center">
    <div class="text-xs tracking-widest text-neutral-500 uppercase">Song complete</div>
    <h2 class="text-2xl font-bold">{room.song?.name}</h2>
    <div class="text-sm text-neutral-400">{room.song?.artist}</div>
    <div class="mt-2 font-mono text-3xl font-bold text-primary">{total.toLocaleString()}</div>
    <div class="text-xs text-neutral-500">band score</div>
  </div>

  {#if timing && timingOpen}
    <div class="flex w-full max-w-xl items-center gap-3 rounded-xl border border-white/10 bg-white/[0.04] px-4 py-2.5 text-sm">
      {#if timing.adjusted !== null}
        <span class="text-neutral-300">
          Your hits averaged {describeLate(timing.late)}. Audio offset moved {timing.adjusted > 0 ? '+' : ''}{timing.adjusted} ms.
        </span>
        <button class="ml-auto text-xs text-neutral-400 hover:text-white" onclick={() => (timingOpen = false)}>OK</button>
      {:else}
        <span class="text-neutral-300">Your hits averaged {describeLate(timing.late)}. Adjust audio offset?</span>
        <button
          class="ml-auto rounded-lg bg-white px-3 py-1 text-xs font-semibold text-black hover:bg-neutral-200"
          onclick={applyTiming}>Apply</button
        >
        <button class="text-xs text-neutral-400 hover:text-white" onclick={() => (timingOpen = false)}>Not now</button>
      {/if}
    </div>
  {/if}

  {#if kept}
    <div
      class="flex w-full max-w-xl flex-wrap items-center gap-x-5 gap-y-2 rounded-xl border px-4 py-3 text-sm {newBest
        ? 'border-primary/40 bg-primary/10'
        : 'border-white/10 bg-white/[0.04]'}"
      data-testid="encore-kept-run"
    >
      <div>
        <div class="font-semibold {newBest ? 'text-primary' : ''}">{newBest ? 'New personal best' : 'Personal best'}</div>
        <div class="flex items-center gap-1.5 text-xs text-neutral-400">
          <span class="font-mono text-white">{kept.best.score.toLocaleString()}</span>
          <Stars stars={kept.best.stars} />
          {#if kept.best.full_combo}<FullCombo />{/if}
        </div>
      </div>
      {#if newBest}
        <div class="text-xs text-neutral-400">
          <div>Previous best</div>
          <div class="font-mono text-white">{kept.previous ? kept.previous.score.toLocaleString() : 'None yet'}</div>
        </div>
      {/if}
      <div class="ml-auto text-right text-xs text-neutral-400">
        <div>Friends <span class="font-mono text-white">{describe(kept.friends)}</span></div>
        {#if kept.everyone}
          <div>Everyone <span class="font-mono text-white">{describe(kept.everyone)}</span></div>
        {/if}
      </div>
    </div>
  {/if}

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
