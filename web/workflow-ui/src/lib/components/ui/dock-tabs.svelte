<script lang="ts">
  import type { WorkflowResource } from '@tawks/workflow-types';
  import { setContext } from 'svelte';
  import WorkflowDisplay from '../workflow-display.svelte';

  let {
    workflow,
    visuals,
    layout,
  }: { workflow: WorkflowResource; visuals: any[]; layout: 'wide' | 'phone' } = $props();

  setContext('dock-tabs', layout);

  function displayKey(display: any, index: number) {
    const typeKey =
      typeof display?.type === 'string'
        ? display.type
        : Object.keys(display?.type ?? {})
            .sort()
            .join('+');
    return `${index}:${typeKey}`;
  }
</script>

{#each visuals as visual (visual.id)}
  {#each visual.display.Sticky.content as display, index (displayKey(display, index))}
    <WorkflowDisplay from="dock-tabs" {workflow} {display} inputs={workflow.responses} />
  {/each}
{/each}
