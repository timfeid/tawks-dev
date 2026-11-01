<script lang="ts">
  import type { WorkflowResource } from '@tawks/workflow-types';
  import * as look from '../../look';
  import WorkflowDisplay from '../workflow-display.svelte';

  let { workflow, actions }: { workflow: WorkflowResource; actions: any[] } = $props();

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

{#if actions.length}
  <div class={look.dock.bar}>
    <div class={look.dock.actions}>
      {#each actions as visual (visual.id)}
        {#each visual.display.Sticky.content as display, index (displayKey(display, index))}
          <WorkflowDisplay from="dock-actions" {workflow} {display} inputs={workflow.responses} />
        {/each}
      {/each}
    </div>
  </div>
{/if}
