<script lang="ts">
  import { getContext, untrack } from 'svelte';
  import type { Props } from '../workflow-input.svelte';
  import type { WorkflowStateProvider } from '../../workflow';

  let { workflow, input, inputs }: Props = $props();
  const provider = getContext<WorkflowStateProvider>('workflow-provider');

  let firedFor = '';

  $effect(() => {
    if (typeof input.input.type === 'string' || !('ServerActionLoader' in input.input.type)) return;
    const target = input.input.type.ServerActionLoader.target;
    const step = `${workflow.instance_id}:${workflow.current_node_id}:${input.id}:${target}`;
    if (step === firedFor) return;
    firedFor = step;
    untrack(() => provider.processAction?.(workflow.instance_id, target, inputs));
  });
</script>

<span class="sr-only">Loading</span>
