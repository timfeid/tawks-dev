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
  <div class="sticky bottom-0 left-0 right-0 flex md:left-[var(--sidebar-width)]">
    {#each visuals as visual (visual.id)}
      {#if 'Sticky' in visual.display}
        {#each visual.display.Sticky.content as display, index (displayKey(display, index))}
          <WorkflowDisplay from="wf-footer" {workflow} {display} inputs={workflow.responses} />
        {/each}
      {/if}
    {/each}
  </div>
{/if}
