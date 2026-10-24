<script lang="ts">
  import { setContext, type Snippet } from 'svelte';
  import type { WorkflowStateProvider } from '../../workflow';

  let { provider, children }: { provider: WorkflowStateProvider; children: Snippet } = $props();

  setContext('workflow-provider', provider);
</script>

{@render children()}
