<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { HTMLOlAttributes } from 'svelte/elements';
  import { cn } from '../utils.js';

  let {
    children,
    class: className,
    ...restProps
  }: HTMLOlAttributes & { children: Snippet } = $props();
</script>

<ol
  class={cn(
    'flex flex-wrap items-center gap-1 text-xs lowercase text-muted-foreground break-words',
    className,
  )}
  {...restProps}
>
  {@render children()}
</ol>
