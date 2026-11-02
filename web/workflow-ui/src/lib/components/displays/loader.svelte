<script lang="ts">
  import { PageLoader } from '@tawks/ui/ready-gate';
  import type { Props } from '../workflow-display.svelte';

  let {}: Props = $props();

  const SHOW_AFTER_MS = 500;

  let show = $state(false);

  $effect(() => {
    const timer = setTimeout(() => (show = true), SHOW_AFTER_MS);
    return () => clearTimeout(timer);
  });
</script>

<!-- Holds its height so nothing moves when the ghost appears. -->
<div class="flex min-h-[4.75rem] items-center justify-center p-4">
  {#if show}<PageLoader />{/if}
</div>
