<script lang="ts">
  import type { LyricLine } from '../lib/types';

  let { lines, time, top = '4rem' }: { lines: LyricLine[]; time: number; top?: string } = $props();

  /** A line shows a little before it is sung and stays until the next one is due. */
  const LEAD_MS = 1_500;

  const current = $derived.by(() => {
    let index = -1;
    for (let i = 0; i < lines.length; i++) {
      if (lines[i].start - LEAD_MS > time) break;
      index = i;
    }
    if (index < 0) return -1;
    const next = lines[index + 1];
    const over = time > lines[index].end + 400 && (!next || next.start - LEAD_MS > time);
    return over ? -1 : index;
  });
  const upcoming = $derived(current >= 0 ? lines[current + 1] : lines.find((l) => l.start - LEAD_MS * 2 <= time && l.start > time));
</script>

{#snippet line(entry: LyricLine, live: boolean)}
  <div class={live ? 'text-xl font-semibold md:text-2xl' : 'mt-1 text-sm text-white/40'}>
    {#each entry.words as word, i (i)}<span
        class={live ? (word.t <= time ? 'text-primary' : 'text-white/80') : ''}
        >{word.text}</span
      >{#if !word.joins && i < entry.words.length - 1}{' '}{/if}{/each}
  </div>
{/snippet}

<div
  class="pointer-events-none absolute left-1/2 w-[min(90%,720px)] -translate-x-1/2 text-center drop-shadow-[0_2px_4px_rgba(0,0,0,0.9)]"
  style="top: {top}"
>
  {#if current >= 0}
    {@render line(lines[current], true)}
  {/if}
  {#if upcoming && upcoming !== lines[current]}
    {@render line(upcoming, false)}
  {/if}
</div>
