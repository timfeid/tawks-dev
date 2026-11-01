<script lang="ts">
  import { Check, VolumeX } from '@lucide/svelte';
  import type { RoomConnection } from '../lib/room.svelte';
  import { INSTRUMENT_LABELS } from '../lib/types';

  let { conn, live = false, compact = false }: { conn: RoomConnection; live?: boolean; compact?: boolean } = $props();

  const players = $derived(conn.room?.players ?? []);
</script>

<div class="space-y-1.5">
  {#if !compact}
    <div class="text-xs font-semibold tracking-wider text-neutral-500 uppercase">Band</div>
  {/if}
  {#each players as player (player.conn)}
    <div class="flex items-center gap-2.5 rounded-lg bg-white/[0.04] px-2.5 py-1.5 {player.progress.muted && live ? 'opacity-60' : ''}">
      {#if player.avatar_url}
        <img src={player.avatar_url} alt="" class="size-7 shrink-0 rounded-md object-cover" />
      {:else}
        <div class="size-7 shrink-0 rounded-md bg-white/10"></div>
      {/if}
      <div class="min-w-0 flex-1">
        <div class="truncate text-sm font-medium">
          {player.name}{player.conn === conn.conn ? ' (you)' : ''}
        </div>
        <div class="truncate text-xs text-neutral-400">
          {#if player.away}
            Reconnecting...
          {:else if player.part}
            {INSTRUMENT_LABELS[player.part.instrument]}, <span class="capitalize">{player.part.difficulty}</span>
          {:else}
            Watching
          {/if}
        </div>
      </div>
      {#if live && player.playing}
        {#if player.progress.muted}<VolumeX class="size-3.5 text-red-400" />{/if}
        <div class="text-right">
          <div class="font-mono text-sm">{player.progress.score.toLocaleString()}</div>
          <div class="font-mono text-[11px] text-neutral-400">
            {player.progress.multiplier}x / {player.progress.streak}{player.progress.star_active ? ' / star' : ''}
          </div>
        </div>
      {:else if !live && player.ready}
        <span class="flex items-center gap-1 text-xs font-semibold text-primary"><Check class="size-3.5" /> Ready</span>
      {/if}
    </div>
  {/each}
</div>
