<script lang="ts">
  import type { WorkflowResource } from '@tawks/workflow-types';
  import WorkflowDisplay from '../workflow-display.svelte';

  let {
    workflow,
    visuals,
  }: {
    workflow: WorkflowResource;
    visuals: any[];
  } = $props();

  function displayKey(display: any, index: number) {
    const typeKey =
      typeof display?.type === 'string'
        ? display.type
        : Object.keys(display?.type ?? {})
            .sort()
            .join('+');
    return `${index}:${typeKey}:${display?.class ?? ''}`;
  }
</script>

{#if visuals.length > 0}
  <div class="bg-background sticky top-0 z-10">
    <div class="mx-auto flex w-full p-3">
      {#each visuals as visual (visual.id)}
        {#if 'Sticky' in visual.display}
          {#each visual.display.Sticky.content as display, index (displayKey(display, index))}
            <WorkflowDisplay from="wf-header" {workflow} {display} inputs={workflow.responses} />
          {/each}
        {/if}
      {/each}
    </div>
  </div>
{/if}
