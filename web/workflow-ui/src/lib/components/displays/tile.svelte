<script lang="ts">
  import WorkflowShell from '../ui/workflow-shell.svelte';
  import { getNestedValue } from '../../utils.js';
  import type { Props } from '../workflow-display.svelte';
  import type { WorkflowResource } from '@tawks/workflow-types';
  import { getContext } from 'svelte';
  import type { WorkflowStateProvider } from '../../workflow';

  let { display, inputs }: Props = $props();

  const provider = getContext<WorkflowStateProvider>('workflow-provider');

  let workflowToShow = $derived.by(() => {
    if (typeof display.type === 'string' || !('Tile' in display.type)) return null;
    const template = display.type.Tile.workflow;
    if (typeof template !== 'string') return null;

    const val = getNestedValue(inputs, template);
    if (!val || typeof val !== 'object') return null;

    return val as WorkflowResource;
  });
</script>

{#if workflowToShow && provider}
  <WorkflowShell workflow={workflowToShow} {provider} />
{/if}