<script lang="ts">
  import { Trophy } from '@lucide/svelte';
  import type { EndReason, Placements, Seat } from './contract';

  let {
    placements,
    reason = null,
    roster,
    mySeat,
    colorOf,
  }: { placements: Placements; reason?: EndReason | null; roster: Seat[]; mySeat: number | null; colorOf: (seat: number) => string } = $props();

  const nameOf = (seat: number) => roster.find((member) => member.seat === seat)?.name ?? `Seat ${seat}`;
  const ordinal = (place: number) => ['1st', '2nd', '3rd'][place] ?? `${place + 1}th`;
  /** Ties share a place, and the next place skips the ones they took. */
  const rows = $derived.by(() => {
    const out: { place: number; seat: number }[] = [];
    let place = 0;
    for (const tier of placements) {
      for (const seat of tier) out.push({ place, seat });
      place += tier.length;
    }
    return out;
  });
  const headline = $derived.by(() => {
    if (reason === 'crashed') return 'The game stopped';
    if (reason === 'timeout') return 'Out of time';
    const winners = placements[0] ?? [];
    if (mySeat !== null && winners.includes(mySeat)) return winners.length > 1 ? 'You tied for first' : 'You win';
    if (winners.length > 1) return 'A tie';
    return winners.length ? `${nameOf(winners[0])} wins` : 'Game over';
  });
</script>

<div class="made-podium pointer-events-none absolute right-4 top-1/2 w-64 -translate-y-1/2 p-4 max-md:top-auto max-md:bottom-3 max-md:right-1/2 max-md:translate-x-1/2 max-md:translate-y-0 text-center text-white" role="status" aria-live="polite">
  <div class="flex items-center justify-center gap-2 text-2xl font-black text-amber-300"><Trophy class="size-5" />{headline}</div>
  <ol class="mt-3 flex flex-col gap-1 text-left text-sm">
    {#each rows as row (row.seat)}
      <li class="flex items-center gap-2" class:font-bold={row.seat === mySeat}>
        <span class="w-9 font-bold text-white/70">{ordinal(row.place)}</span>
        <span class="size-3 rounded-full" style="background:{colorOf(row.seat)}"></span>
        <span class="truncate">{nameOf(row.seat)}</span>
      </li>
    {/each}
  </ol>
</div>

<style>
  .made-podium {
    border-top: 2px solid #fcd34d;
    background: linear-gradient(180deg, rgb(4 12 24 / 0.92), rgb(4 12 24 / 0.75));
    clip-path: polygon(0.6rem 0, calc(100% - 0.6rem) 0, 100% 0.6rem, 100% 100%, 0 100%, 0 0.6rem);
  }
</style>
