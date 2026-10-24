<script lang="ts">
  import type { MatchStateView, PlayerState } from '../types';

  let {
    matchState,
    players = [],
    viewerId = null,
  }: {
    matchState: MatchStateView;
    players: PlayerState[];
    viewerId: string | null;
  } = $props();

  const viewer = $derived(players.find((p) => p.id === viewerId));
  const opponents = $derived(players.filter((p) => p.id !== viewerId));

  // Show as "You 1 – 0 Opp" from the viewer's perspective.
  const viewerWins = $derived(
    viewer ? (matchState.wins[viewer.id] ?? 0) : 0,
  );
  const oppWins = $derived(
    opponents[0] ? (matchState.wins[opponents[0].id] ?? 0) : 0,
  );
</script>

<div class="pointer-events-none absolute left-1/2 top-4 z-40 -translate-x-1/2">
  <div class="flex items-center gap-3 rounded-full border border-amber-500/40 bg-slate-950/90 px-4 py-1.5 font-mono text-sm uppercase tracking-widest text-slate-200 shadow-[0_0_20px_rgba(251,191,36,0.2)] backdrop-blur-md">
    <span class="text-[10px] text-amber-400">
      Bo{matchState.wins_required * 2 - 1}
    </span>
    <span class="text-[10px] text-slate-500">
      Game {matchState.current_game}
    </span>
    <div class="h-3 w-px bg-slate-700"></div>
    <div class="flex items-center gap-1.5">
      {#each Array(matchState.wins_required) as _, i}
        <div
          class="h-2 w-2 rounded-full border"
          class:border-green-400={i < viewerWins}
          class:bg-green-400={i < viewerWins}
          class:border-slate-600={i >= viewerWins}
        ></div>
      {/each}
      <span class="mx-1 text-[10px] text-slate-500">You</span>
      <span class="font-bold text-white">{viewerWins}</span>
      <span class="text-slate-500">–</span>
      <span class="font-bold text-white">{oppWins}</span>
      <span class="mx-1 text-[10px] text-slate-500">Opp</span>
      {#each Array(matchState.wins_required) as _, i}
        <div
          class="h-2 w-2 rounded-full border"
          class:border-red-400={i < oppWins}
          class:bg-red-400={i < oppWins}
          class:border-slate-600={i >= oppWins}
        ></div>
      {/each}
    </div>
  </div>
</div>
