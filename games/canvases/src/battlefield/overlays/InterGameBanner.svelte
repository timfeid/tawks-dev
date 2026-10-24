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

  // We're in SideboardSwap, entered right after a game's winner was
  // recorded. current_game is the game that just finished (the next
  // game's counter only bumps when start_next_game_in_match runs).
  const gameJustEnded = $derived(matchState.current_game);

  const opponent = $derived(players.find((p) => p.id !== viewerId));
  const viewerWins = $derived(viewerId ? (matchState.wins[viewerId] ?? 0) : 0);
  const oppWins = $derived(opponent ? (matchState.wins[opponent.id] ?? 0) : 0);

  // Authoritative per-game history — the last entry is the most recent
  // game's winner. Falls back to total-comparison if game_winners is
  // missing (older server state).
  const lastGameWinner = $derived(
    matchState.game_winners?.length
      ? matchState.game_winners[matchState.game_winners.length - 1]
      : viewerWins > oppWins
        ? (viewerId ?? '')
        : (opponent?.id ?? ''),
  );
  const viewerWonLast = $derived(lastGameWinner === viewerId);

  // Brief auto-dismiss: show for 2.5s then fade, giving the sideboard
  // overlay the stage.
  let visible = $state(true);
  let fading = $state(false);
  $effect(() => {
    const t1 = setTimeout(() => (fading = true), 2200);
    const t2 = setTimeout(() => (visible = false), 2800);
    return () => {
      clearTimeout(t1);
      clearTimeout(t2);
    };
  });
</script>

{#if visible}
  <div
    class="pointer-events-none fixed inset-x-0 top-20 z-[60] flex justify-center transition-opacity duration-500"
    class:opacity-0={fading}
  >
    <div
      class="flex min-w-[320px] flex-col items-center rounded-lg border-2 bg-slate-950/95 px-10 py-4 shadow-[0_0_40px_rgba(0,0,0,0.6)] backdrop-blur-md"
      class:border-green-400={viewerWonLast}
      class:border-red-400={!viewerWonLast}
    >
      <div class="font-mono text-xs uppercase tracking-widest text-slate-400">
        Game {gameJustEnded}
      </div>
      <div
        class="mt-1 text-4xl font-black uppercase tracking-wider"
        class:text-green-400={viewerWonLast}
        class:text-red-400={!viewerWonLast}
      >
        {viewerWonLast ? 'You Won' : 'You Lost'}
      </div>
      <div class="mt-2 font-mono text-sm text-slate-300">
        Series: {viewerWins} – {oppWins}
      </div>
    </div>
  </div>
{/if}
