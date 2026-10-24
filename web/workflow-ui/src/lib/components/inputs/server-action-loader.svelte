<script lang="ts">
  import { onMount, getContext } from 'svelte';
  import type { Props } from '../workflow-input.svelte';
  import type { WorkflowStateProvider } from '../../workflow';

  let { workflow, input, inputs }: Props = $props();
  const provider = getContext<WorkflowStateProvider>('workflow-provider');

  onMount(async () => {
    if (typeof input.input.type !== 'string' && 'ServerActionLoader' in input.input.type) {
      if (provider.processAction) {
          await provider.processAction(
              workflow.instance_id,
              input.input.type.ServerActionLoader.target,
              inputs
          );
      }
    }
  });
</script>

<div class="flex items-center justify-center p-4 italic text-muted-foreground text-xs">
    Loading data...
</div>