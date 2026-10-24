<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { HTMLAttributes } from 'svelte/elements';
  import { cn } from '../utils.js';

  let {
    children,
    class: className,
    ...restProps
  }: HTMLAttributes<HTMLSpanElement> & { children: Snippet } = $props();
</script>

<span
  role="link"
  aria-disabled="true"
  aria-current="page"
  class={cn('font-normal text-lime-400/70', className)}
  {...restProps}
>
  {@render children()}
</span>
