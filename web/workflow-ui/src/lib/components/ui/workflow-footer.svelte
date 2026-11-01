<script lang="ts">
  import type { WorkflowResource } from '@tawks/workflow-types';
  import * as look from '../../look';
  import WorkflowDisplay from '../workflow-display.svelte';
  import WorkflowDock from './workflow-dock.svelte';

  let {
    workflow,
    visuals,
    tabs = [],
    actions = [],
  }: {
    workflow: WorkflowResource;
    visuals: any[];
    tabs?: any[];
    actions?: any[];
  } = $props();

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

{#if visuals.length || tabs.length || actions.length}
  <div class={[look.dock.shell, actions.length && look.dock.floating]}>
    {#if visuals.length}
      <div class="pointer-events-auto flex">
        {#each visuals as visual (visual.id)}
          {#if 'Sticky' in visual.display}
            {#each visual.display.Sticky.content as display, index (displayKey(display, index))}
              <WorkflowDisplay from="wf-footer" {workflow} {display} inputs={workflow.responses} />
            {/each}
          {/if}
        {/each}
      </div>
    {/if}
    <WorkflowDock {workflow} {tabs} {actions} />
  </div>
{/if}
