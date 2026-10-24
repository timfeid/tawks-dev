<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import type { Props } from '../workflow-display.svelte';
  import { cn } from '../../utils';

  let { display }: Props = $props();

  let timeout: any;
  let show = $state(false);
  
  onMount(() => {
    timeout = setTimeout(() => {
      show = true;
    }, 250);
  });

  onDestroy(() => {
    if (timeout) clearTimeout(timeout);
  });
</script>

<div class={cn('flex items-center justify-center p-4', display.class)}>
  {#if show}
    <div class="flex items-center gap-2 text-primary animate-pulse">
        <span class="text-sm font-medium">Loading...</span>
    </div>
  {/if}
</div>