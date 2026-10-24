<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { HTMLAttributes } from 'svelte/elements';

  let {
    children,
    class: className = '',
    ...restProps
  }: HTMLAttributes<HTMLElement> & { children: Snippet } = $props();
</script>

<nav aria-label="breadcrumb" class={className} {...restProps}>
  {@render children()}
</nav>
