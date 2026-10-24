<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { HTMLLiAttributes } from 'svelte/elements';
  import { cn } from '../utils.js';

  let {
    children,
    class: className,
    ...restProps
  }: HTMLLiAttributes & { children?: Snippet } = $props();
</script>

<li
  role="presentation"
  aria-hidden="true"
  class={cn('text-neutral-600', className)}
  {...restProps}
>
  {#if children}
    {@render children()}
  {:else}
    /
  {/if}
</li>
