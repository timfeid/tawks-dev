<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { HTMLLiAttributes } from 'svelte/elements';
  import { cn } from '../utils.js';

  let {
    children,
    class: className,
    ...restProps
  }: HTMLLiAttributes & { children: Snippet } = $props();
</script>

<li class={cn('inline-flex items-center gap-1.5', className)} {...restProps}>
  {@render children()}
</li>
