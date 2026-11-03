<script lang="ts">
  import { setContext, type Snippet } from 'svelte';

  /** One row of a list: inputs inside read and clear per row. */
  let { children }: { children: Snippet } = $props();
  setContext('workflow-row', true);
</script>

{@render children()}
