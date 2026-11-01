<script lang="ts">
  import type { Item } from './protocol';
  import { ITEMS } from './board/items';
  import { ITEM_ICONS } from './board/item-icons';

  /** An item's icon in its colour, the same one its badge carries on the board. */
  let { item, class: className = 'size-4' }: { item: Item; class?: string } = $props();
</script>

<svg
  class="shrink-0 {className}"
  viewBox="0 0 24 24"
  fill="none"
  stroke={ITEMS[item].color}
  stroke-width="2.2"
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
>
  {#each ITEM_ICONS[item] as shape, i (i)}
    {#if 'd' in shape}<path d={shape.d} />{:else}<circle cx={shape.cx} cy={shape.cy} r={shape.r} />{/if}
  {/each}
</svg>
