<script lang="ts">
  import { Star } from '@lucide/svelte';
  import { MAX_STARS } from '../lib/types';

  let { stars, size = 'size-3' }: { stars: number; size?: string } = $props();
</script>

<span class="inline-flex items-center gap-px" title="{stars} of {MAX_STARS} stars" aria-label="{stars} of {MAX_STARS} stars">
  {#each Array.from({ length: MAX_STARS }, (_, i) => i < stars) as lit, i (i)}
    <Star class="{size} {lit ? 'fill-amber-300 text-amber-300' : 'text-white/15'}" />
  {/each}
</span>
